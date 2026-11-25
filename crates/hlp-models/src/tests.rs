use super::*;
use crate::manifest::{Artifact, Format, COMPONENTS};
use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{header, HeaderMap, Response, StatusCode},
    routing::get,
    Router,
};
use futures_util::stream;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    convert::Infallible,
    sync::{Arc, Mutex},
};

#[test]
fn all_platform_pins_are_immutable() {
    let mut entries = BTreeSet::new();
    for c in COMPONENTS.iter() {
        assert!(entries.insert((&c.platform, &c.id)));
        assert!(matches!(c._kind.as_str(), "model" | "tool"));
        assert!(matches!(
            c.group.as_str(),
            "subtitles" | "urls" | "optional"
        ));
        assert!(!c.files.is_empty());
        for f in &c.files {
            assert!(f.bytes > 0);
            assert_eq!(f.sha256.len(), 64);
            assert!(f.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
            for url in std::iter::once(&f.url).chain(f.mirror.iter()) {
                assert!(url.starts_with("https://"));
                assert!(!["/latest", "/main/", "/master/"]
                    .iter()
                    .any(|s| url.contains(s)));
            }
        }
    }
    let base = required_ids(false);
    let with_urls = required_ids(true);
    if cfg!(any(windows, target_os = "macos")) {
        assert!(!base.contains(&"yt-dlp".into()));
        assert!(with_urls.contains(&"yt-dlp".into()));
        assert!(with_urls.contains(&"deno".into()));
        assert!(!with_urls.contains(&"asr-qwen3-0.6b-mlx".into()));
    }
}

#[test]
fn version_marker_accepts_stable_tags_only() {
    assert!(update::valid_version("2026.10.26.1"));
    for value in [
        "..",
        "x",
        "12/../../bin",
        "2..3",
        "/usr/bin",
        "",
        " 2026.10",
    ] {
        assert!(!update::valid_version(value))
    }
}

#[test]
fn previously_extracted_windows_model_is_still_ready() {
    let model = COMPONENTS
        .iter()
        .find(|c| c.id == "asr-qwen3-0.6b-sherpa")
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join(&model.directory);
    for artifact in &model.files {
        let destination = directory.join(artifact.name.as_ref().unwrap());
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        File::create(destination)
            .unwrap()
            .set_len(artifact.bytes)
            .unwrap();
    }
    assert!(ready(root.path(), model));
    fs::write(directory.join("tokenizer/vocab.json"), b"partial").unwrap();
    assert!(!ready(root.path(), model));
}

#[derive(Clone)]
struct Origin {
    requests: Arc<Mutex<Vec<String>>>,
    stream_first: bool,
    honor_ranges: bool,
}
const PAYLOAD: &[u8] = b"123456789";
async fn response(headers: HeaderMap, State(state): State<Origin>) -> Response<Body> {
    let range = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    state.requests.lock().unwrap().push(range.clone());
    if state.stream_first && range.is_empty() {
        let parts = stream::unfold(0, |step| async move {
            match step {
                0 => Some((Ok::<_, Infallible>(Bytes::from_static(&PAYLOAD[..3])), 1)),
                1 => {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    Some((Ok(Bytes::from_static(&PAYLOAD[3..])), 2))
                }
                _ => None,
            }
        });
        return Response::new(Body::from_stream(parts));
    }
    if state.honor_ranges && range == "bytes=3-" {
        Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_RANGE, "bytes 3-8/9")
            .body(Body::from(&PAYLOAD[3..]))
            .unwrap()
    } else {
        Response::new(Body::from(PAYLOAD))
    }
}
async fn fixture(
    stream_first: bool,
    honor_ranges: bool,
) -> (Artifact, Origin, tokio::task::JoinHandle<()>) {
    let state = Origin {
        requests: Default::default(),
        stream_first,
        honor_ranges,
    };
    let app = Router::new()
        .route("/", get(response))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let artifact = Artifact {
        url,
        mirror: None,
        bytes: PAYLOAD.len() as u64,
        sha256: hex::encode(Sha256::digest(PAYLOAD)),
        name: Some("file".into()),
        format: Format::Raw,
        prefix: String::new(),
        members: Default::default(),
    };
    (artifact, state, server)
}
#[test]
fn model_source_uses_region_instead_of_interface_language() {
    for locale in ["en-CN", "zh-Hans-CN", "zh_CN.UTF-8"] {
        assert!(transfer::mainland_region(locale));
    }
    for locale in [
        "zh-TW",
        "zh-HK",
        "zh-Hans-SG",
        "en-US",
        "zh",
        "en-US-u-rg-cnzzzz",
    ] {
        assert!(!transfer::mainland_region(locale));
    }
}

#[tokio::test]
async fn automatic_model_source_honors_region_and_falls_back_on_failure() {
    let (mut artifact, _, origin) = fixture(false, false).await;
    let (other, _, mirror) = fixture(false, false).await;
    artifact.mirror = Some(other.url);
    let client = transfer::client().unwrap();
    let cancel = CancellationToken::new();
    assert!(!transfer::prefer_mirror(&client, &artifact, false, &cancel)
        .await
        .unwrap());
    assert!(transfer::prefer_mirror(&client, &artifact, true, &cancel)
        .await
        .unwrap());
    artifact.url.push_str("unavailable");
    assert!(transfer::prefer_mirror(&client, &artifact, false, &cancel)
        .await
        .unwrap());
    cancel.cancel();
    assert!(transfer::prefer_mirror(&client, &artifact, true, &cancel)
        .await
        .is_err());
    origin.abort();
    mirror.abort();
}

#[tokio::test]
async fn cancelled_download_resumes_at_verified_offset() {
    let (f, origin, server) = fixture(true, true).await;
    let stage = tempfile::tempdir().unwrap();
    let partial = transfer::part_path(stage.path(), &f);
    let cancel = CancellationToken::new();
    let monitor = {
        let cancel = cancel.clone();
        let path = partial.clone();
        tokio::spawn(async move {
            tokio::time::timeout(Duration::from_secs(3), async {
                while file_len(&path) != 3 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            cancel.cancel();
        })
    };
    let client = transfer::client().unwrap();
    let result = transfer::obtain(&client, &f, stage.path(), |_| {}, || {}, &cancel).await;
    monitor.await.unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read(&partial).unwrap(), &PAYLOAD[..3]);
    let complete = transfer::obtain(
        &client,
        &f,
        stage.path(),
        |_| {},
        || {},
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(fs::read(complete).unwrap(), PAYLOAD);
    assert_eq!(*origin.requests.lock().unwrap(), ["", "bytes=3-"]);
    server.abort();
}
#[tokio::test]
async fn a_server_ignoring_range_replaces_the_partial() {
    let (a, state, server) = fixture(false, false).await;
    let stage = tempfile::tempdir().unwrap();
    fs::write(transfer::part_path(stage.path(), &a), b"old").unwrap();
    let file = transfer::obtain(
        &transfer::client().unwrap(),
        &a,
        stage.path(),
        |_| {},
        || {},
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(fs::read(file).unwrap(), PAYLOAD);
    assert_eq!(*state.requests.lock().unwrap(), ["bytes=3-"]);
    server.abort();
}
#[tokio::test]
async fn bad_complete_digest_is_repaired() {
    let (a, state, server) = fixture(false, false).await;
    let stage = tempfile::tempdir().unwrap();
    fs::write(
        transfer::part_path(stage.path(), &a),
        b"bad digest"[..9].to_vec(),
    )
    .unwrap();
    let file = transfer::obtain(
        &transfer::client().unwrap(),
        &a,
        stage.path(),
        |_| {},
        || {},
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(fs::read(file).unwrap(), PAYLOAD);
    assert_eq!(*state.requests.lock().unwrap(), [""]);
    server.abort();
}
#[tokio::test]
async fn two_callers_publish_one_verified_copy() {
    let (mut a, state, server) = fixture(false, false).await;
    a.name = Some("tokenizer/vocab.json".into());
    let root = tempfile::tempdir().unwrap();
    let c = Component {
        id: "test".into(),
        platform: "both".into(),
        group: "subtitles".into(),
        _kind: "model".into(),
        directory: "model/test".into(),
        ready: Check::File,
        files: vec![a],
        repo: None,
        commands: vec![],
    };
    let client = transfer::client().unwrap();
    let cancel = CancellationToken::new();
    let call = || install::component(root.path(), &c, &client, &|_| {}, &cancel);
    let (one, two) = tokio::join!(call(), call());
    one.unwrap();
    two.unwrap();
    assert_eq!(
        fs::read(root.path().join("model/test/tokenizer/vocab.json")).unwrap(),
        PAYLOAD
    );
    assert_eq!(state.requests.lock().unwrap().len(), 1);
    server.abort();
}

/// Complete an actual network install without touching a user's tool or model directories.
#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "downloads native tools, Silero and the translation runtime"]
async fn real_managed_install_and_explicit_update() {
    let root = tempfile::tempdir().unwrap();
    let client = transfer::client().unwrap();
    let cancel = CancellationToken::new();
    for id in [
        "vad-silero",
        "ffmpeg",
        "llama-runtime-b11429",
        "yt-dlp",
        "deno",
    ] {
        let c = component(id).unwrap();
        assert!(!ready(root.path(), c));
        install::component(root.path(), c, &client, &|_| {}, &cancel)
            .await
            .unwrap();
        assert!(ready(root.path(), c), "{id}");
    }
    update::update(root.path(), &|_| {}, &cancel).await.unwrap();
    assert!(ready(root.path(), component("yt-dlp").unwrap()));
}

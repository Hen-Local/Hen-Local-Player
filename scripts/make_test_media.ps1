# Windows SAPI synthesis adapted from Hen-Local-Translator/scripts/make_e2e_wavs.ps1.
# Keep this script UTF-8 with BOM so Windows PowerShell 5.1 reads all seven scripts correctly.
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/build_env.ps1"
$ffmpeg = Get-HlpFfmpeg
$utf8 = New-Object System.Text.UTF8Encoding($false)
$culture = [Globalization.CultureInfo]::InvariantCulture
$fixtures = Join-Path $repoDirectory 'tests/fixtures'
$tempDir = Join-Path ([IO.Path]::GetTempPath()) ('hlp-sapi-' + [Guid]::NewGuid().ToString('N'))
$video = @('-f', 'lavfi', '-i', 'testsrc2=size=1280x720:rate=30')
$linesByLanguage = [ordered]@{
    en = 'Today the sky is clear and blue.|We walk to the station in the morning.|The small house is beside the river.|The children are playing in the garden.|We cook dinner together every evening.|Tomorrow we will take the train home.'
    ja = '今日はとてもいい天気です。|駅の前で友達と会いました。|私たちは公園まで歩きました。|小さな子供が遊んでいます。|夕方には家に帰ります。|明日もここに来たいです。'
    fr = 'Le soleil brille dans le ciel.|Nous allons au marché ce matin.|La petite maison est près de la rivière.|Les enfants jouent dans le jardin.|Je prépare le dîner avec ma famille.|Demain nous irons à la plage.'
    de = 'Heute scheint die Sonne.|Wir gehen zusammen in den Park.|Der kleine Hund spielt im Garten.|Meine Schwester liest ein Buch.|Am Abend kochen wir zusammen.|Morgen fahren wir mit dem Zug.'
    es = 'Hoy hace muy buen tiempo.|Vamos al mercado por la mañana.|Mi hermana vive cerca del río.|Los niños juegan en el parque.|Esta noche cenamos en casa.|Mañana iremos a la playa.'
    zh = '今天的天气很好。|我们早上一起去公园。|河边有一座小房子。|孩子们在花园里玩耍。|晚上我们一起做饭。|明天我们坐火车回家。'
    ko = '오늘은 날씨가 아주 좋습니다.|우리는 아침에 공원에 갑니다.|동생이 재미있는 책을 읽습니다.|아이들이 정원에서 놀고 있습니다.|저녁에는 가족과 함께 식사를 합니다.|내일은 기차를 타고 집에 갑니다.'
}

function Invoke-Ffmpeg([string[]]$Arguments) {
    & $ffmpeg -nostdin -hide_banner -loglevel error -y @Arguments
    if ($LASTEXITCODE) { throw "FFmpeg failed ($LASTEXITCODE) while writing $($Arguments[-1])." }
}

function Find-Encoder([string[]]$Candidates) {
    # An encoder being listed does not mean that its driver and hardware are available.
    foreach ($codec in $Candidates) {
        & $ffmpeg -nostdin -hide_banner -loglevel error -f lavfi -i 'color=s=320x240:r=30' -frames:v 2 -c:v $codec -b:v 300k -pix_fmt yuv420p -f null -
        if ($LASTEXITCODE -eq 0) { return $codec }
    }
    throw "No working FFmpeg encoder among $($Candidates -join ', ')."
}

function Write-Truth($Name, $Language, $Sentences) {
    $truth = [ordered]@{language = $Language; sentences = @($Sentences)}
    $file = Join-Path $fixtures "fixture_$Name.truth.json"
    [IO.File]::WriteAllText($file, ($truth | ConvertTo-Json -Depth 5), $utf8)
}

function Write-SpeechTrack($Synth, $Voice, $Language) {
    $Synth.SelectVoice($Voice)
    $format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(
        16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen,
        [System.Speech.AudioFormat.AudioChannel]::Mono)
    $raw = Join-Path $tempDir 'track.pcm'
    $lineWave = Join-Path $tempDir 'sentence.wav'
    $linePcm = Join-Path $tempDir 'sentence.pcm'
    $stream = [IO.File]::Create($raw)
    $truth = @()
    try {
        $gap = New-Object byte[] 48000
        $stream.Write($gap, 0, 16000)
        foreach ($line in $linesByLanguage[$Language].Split('|')) {
            $Synth.SetOutputToWaveFile($lineWave, $format)
            try { $Synth.Speak($line) } finally { $Synth.SetOutputToNull() }
            Invoke-Ffmpeg @('-i',$lineWave,'-af','silenceremove=start_periods=1:start_threshold=-50dB,areverse,silenceremove=start_periods=1:start_threshold=-50dB,areverse','-ar','16000','-ac','1','-f','s16le',$linePcm)
            $speech = [IO.File]::ReadAllBytes($linePcm)
            if (!$speech.Length) { throw "SAPI voice $Voice produced an empty sentence." }
            $truth += [ordered]@{onsetMs = [int][Math]::Floor($stream.Position / 32); text = $line}
            $stream.Write($speech, 0, $speech.Length)
            $stream.Write($gap, 0, $gap.Length)
        }
        return [pscustomobject]@{ file = $raw; duration = $stream.Length / 32000.0; sentences = $truth }
    } finally {
        $stream.Dispose()
    }
}

function Write-LongFixture($Speech, $AudioArgs, $H264) {
    $loop = Join-Path $tempDir 'loop.mp4'
    Invoke-Ffmpeg ($video + $AudioArgs + @('-c:v',$H264,'-b:v','300k','-pix_fmt','yuv420p','-c:a','aac','-t',$Speech.duration.ToString('F8',$culture),$loop))
    $entries = @()
    $truth = @()
    $duration = $Speech.duration.ToString('F8', $culture)
    for ($i = 0; $i * $Speech.duration -lt 1200; $i++) {
        $entries += "file 'loop.mp4'"
        $entries += "duration $duration"
        foreach ($line in $Speech.sentences) {
            $onset = $i * $Speech.duration * 1000 + $line.onsetMs
            if ($onset + 2000 -lt 1200000) {
                $truth += [ordered]@{ onsetMs = [int][Math]::Round($onset); text = $line.text }
            }
        }
    }
    $list = Join-Path $tempDir 'loop.ffconcat'
    [IO.File]::WriteAllLines($list, [string[]]$entries, $utf8)
    Invoke-Ffmpeg @('-f','concat','-safe','0','-i',$list,'-t','1200','-c','copy',(Join-Path $fixtures 'fixture_long_en.mp4'))
    Write-Truth 'long_en' 'en' $truth
}

Add-Type -AssemblyName System.Speech
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
    $voiceMap = @{}
    foreach ($voice in $synth.GetInstalledVoices()) {
        if (!$voice.Enabled) { continue }
        $lang = $voice.VoiceInfo.Culture.TwoLetterISOLanguageName
        if (!$voiceMap.ContainsKey($lang)) { $voiceMap[$lang] = $voice.VoiceInfo.Name }
    }
    if (!$voiceMap.ContainsKey('en')) { throw 'Install an English Windows SAPI desktop voice, then rerun the fixture generator.' }
    $h264 = Find-Encoder @('h264_nvenc','h264_qsv','h264_amf','h264_mf','libx264','libopenh264')
    $hevc = Find-Encoder @('hevc_nvenc','hevc_qsv','hevc_amf','hevc_mf','libx265','libkvazaar')
    New-Item -ItemType Directory -Path $fixtures, $tempDir -Force | Out-Null
    foreach ($name in $linesByLanguage.Keys) {
        # Keep every container on English-only PCs; the truth file names the actual voice language.
        $lang = if ($voiceMap.ContainsKey($name)) { $name } else { 'en' }
        $voice = $voiceMap[$lang]
        Write-Host "fixture_$name uses $voice ($lang)"
        $speech = Write-SpeechTrack $synth $voice $lang
        $audio = @('-f','s16le','-ar','16000','-ac','1','-i',$speech.file)
        $options = @('-c:v',$h264,'-b:v','900k','-pix_fmt','yuv420p','-c:a','aac')
        $stem = $name
        switch ($name) {
            'en' { $extension = 'm4a'; $inputs = $audio; $options = @('-c:a','aac') }
            'ja' { $extension = 'mp4'; $inputs = $video + $audio }
            'fr' { $extension = 'mkv'; $inputs = $video + $audio; $options[-1] = 'ac3' }
            'de' {
                $extension = 'webm'; $inputs = $video + $audio
                $options = @('-c:v','libvpx-vp9','-b:v','900k','-deadline','realtime','-cpu-used','8','-row-mt','1','-c:a','libopus')
            }
            'es' { $extension = 'avi'; $inputs = $video + $audio; $options[1] = 'mpeg4'; $options[-1] = 'libmp3lame' }
            'zh' { $extension = 'mov'; $inputs = $video + $audio; $options[1] = $hevc; $options += @('-tag:v','hvc1') }
            'ko' {
                $stem = 'ko_51'; $extension = 'mkv'
                $inputs = $video + $audio + @('-f','lavfi','-i','anoisesrc=color=pink:amplitude=0.04:sample_rate=16000')
                $options = @('-filter_complex','[1:a][2:a]amerge=inputs=2,pan=5.1|FL=c1|FR=c1|FC=c0|LFE=c1|BL=c1|BR=c1[a]','-map','0:v','-map','[a]') + $options
            }
        }
        $file = Join-Path $fixtures "fixture_$stem.$extension"
        Invoke-Ffmpeg ($inputs + $options + @('-t',$speech.duration.ToString('F8',$culture),$file))
        Write-Truth $stem $lang $speech.sentences
        if ($name -eq 'en') { Write-LongFixture $speech $audio $h264 }
    }
    Invoke-Ffmpeg ($video + @('-f','lavfi','-i','sine=frequency=440:duration=12','-f','lavfi','-i','anoisesrc=duration=12:amplitude=0.01','-filter_complex','[1:a][2:a]amix[a]','-map','0:v','-map','[a]','-t','12','-c:v',$h264,'-b:v','500k','-c:a','aac',(Join-Path $fixtures 'fixture_music.mp4')))
    Write-Truth 'music' $null @()
} finally {
    $synth.Dispose()
    if (Test-Path -LiteralPath $tempDir) { Remove-Item -LiteralPath $tempDir -Recurse -Force }
}

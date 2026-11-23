# Common Windows developer paths. Dot-source from the repo root or another script.
$repoDirectory = Split-Path -Parent $PSScriptRoot
if (!$env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = 'C:\hlp-target' }
$env:CARGO_TARGET_DIR = [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
if ($env:CARGO_TARGET_DIR -match '\s') { throw 'Choose a CARGO_TARGET_DIR without spaces.' }

function Get-HlpFfmpeg {
    if ($env:HLP_FFMPEG) {
        return (Get-Item -LiteralPath $env:HLP_FFMPEG -ErrorAction Stop).FullName
    }
    $componentFile = Join-Path $repoDirectory 'crates/hlp-models/src/components.json'
    $component = Get-Content -Raw -Encoding UTF8 -LiteralPath $componentFile | ConvertFrom-Json |
        ForEach-Object { $_ } | Where-Object { $_.id -eq 'ffmpeg' -and $_.platform -eq 'windows' }
    $modelDirectory = if ($env:HLP_OMINIX_ROOT) { $env:HLP_OMINIX_ROOT } else { Join-Path $env:USERPROFILE '.OminiX' }
    $managed = Join-Path (Join-Path $modelDirectory $component.directory) 'ffmpeg.exe'
    if (Test-Path -LiteralPath $managed -PathType Leaf) { return $managed }
    if ($env:HLP_ALLOW_SYSTEM_TOOLS -eq '1') { return (Get-Command ffmpeg.exe -CommandType Application -ErrorAction Stop).Source }
    throw 'FFmpeg is missing. Run hlp-cli setup --yes, or set HLP_FFMPEG to the binary.'
}

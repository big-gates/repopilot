# GitLab Generic Package Registry에서 prpilot.exe를 받아
# Windows 어디서든 `prpilot` 호출 가능하도록 사용자 PATH에 설치한다.

param(
  [Parameter(Mandatory = $true)][string]$ProjectId,
  [Parameter(Mandatory = $true)][string]$Tag,
  [string]$GitLabUrl = "https://gitlab.com",
  [string]$PackageName = "prpilot",
  [string]$Token = "",
  [string]$InstallDir = ""
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Token)) {
  if ($env:GITLAB_TOKEN) {
    $Token = $env:GITLAB_TOKEN
  } elseif ($env:GL_TOKEN) {
    $Token = $env:GL_TOKEN
  }
}

$archRaw = $env:PROCESSOR_ARCHITECTURE
switch ($archRaw.ToLower()) {
  "amd64" { $arch = "amd64" }
  "x86" { $arch = "amd64" }
  "arm64" { $arch = "arm64" }
  default { throw "unsupported architecture: $archRaw" }
}

if ([string]::IsNullOrWhiteSpace($InstallDir)) {
  $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\\prpilot"
}

$archiveName = "$PackageName-$Tag-windows-$arch.zip"
$downloadUrl = "$($GitLabUrl.TrimEnd('/'))/api/v4/projects/$ProjectId/packages/generic/$PackageName/$Tag/$archiveName"

$tmpDir = Join-Path $env:TEMP ("prpilot-install-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
  $archivePath = Join-Path $tmpDir $archiveName
  Write-Host "downloading: $downloadUrl"

  $headers = @{}
  if (-not [string]::IsNullOrWhiteSpace($Token)) {
    $headers["PRIVATE-TOKEN"] = $Token
  }

  Invoke-WebRequest -Uri $downloadUrl -Headers $headers -OutFile $archivePath

  Write-Host "extracting package"
  Expand-Archive -Path $archivePath -DestinationPath $tmpDir -Force

  $exePath = Join-Path $tmpDir "prpilot.exe"
  if (-not (Test-Path $exePath)) {
    throw "extracted binary not found: $exePath"
  }

  New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
  Copy-Item $exePath (Join-Path $InstallDir "prpilot.exe") -Force

  $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
  $parts = @()
  if ($userPath) {
    $parts = $userPath -split ';' | Where-Object { $_ -ne "" }
  }

  if (-not ($parts -contains $InstallDir)) {
    $newUserPath = if ([string]::IsNullOrEmpty($userPath)) { $InstallDir } else { "$userPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
    $env:Path = "$InstallDir;$env:Path"
    Write-Host "PATH updated (User): $InstallDir"
  } else {
    Write-Host "PATH already includes: $InstallDir"
  }

  Write-Host "installed: $(Join-Path $InstallDir 'prpilot.exe')"
  Write-Host "check: prpilot --help"
}
finally {
  Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
}

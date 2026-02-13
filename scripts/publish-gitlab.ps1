# 로컬 Windows 머신(러너 없이)에서 prpilot.exe를 빌드하고
# GitLab Generic Package Registry + Release에 업로드한다.
# 기본 동작: git tag 생성/푸시 + 패키지 업로드 + release 생성/업데이트

param(
  [Parameter(Mandatory = $true)][string]$ProjectId,
  [Parameter(Mandatory = $true)][string]$Tag,
  [string]$GitLabUrl = "https://gitlab.com",
  [string]$PackageName = "prpilot",
  [string]$Token = "",
  [switch]$NoTagPush,
  [switch]$NoRelease
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Token)) {
  if ($env:GITLAB_TOKEN) {
    $Token = $env:GITLAB_TOKEN
  } elseif ($env:GL_TOKEN) {
    $Token = $env:GL_TOKEN
  }
}

if ([string]::IsNullOrWhiteSpace($Token)) {
  throw "token is missing. Use -Token or set GITLAB_TOKEN/GL_TOKEN."
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  throw "cargo command not found"
}
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
  throw "git command not found"
}

$archRaw = $env:PROCESSOR_ARCHITECTURE
switch ($archRaw.ToLower()) {
  "amd64" { $arch = "amd64" }
  "x86" { $arch = "amd64" }
  "arm64" { $arch = "arm64" }
  default { throw "unsupported architecture: $archRaw" }
}

$binPath = "target/release/prpilot.exe"
$archiveName = "$PackageName-$Tag-windows-$arch.zip"
$packageUrl = "$($GitLabUrl.TrimEnd('/'))/api/v4/projects/$ProjectId/packages/generic/$PackageName/$Tag/$archiveName"

if (-not $NoTagPush) {
  Write-Host "[1/5] ensuring git tag exists and is pushed"

  if (-not (Test-Path ".git")) {
    throw "current directory is not a git repository (.git missing)"
  }

  git rev-parse --verify --quiet "refs/tags/$Tag" *> $null
  if ($LASTEXITCODE -eq 0) {
    Write-Host "tag exists locally: $Tag"
  } else {
    git tag $Tag
    if ($LASTEXITCODE -ne 0) {
      throw "failed to create local tag: $Tag"
    }
    Write-Host "tag created locally: $Tag"
  }

  git push origin "refs/tags/$Tag" *> $null
  if ($LASTEXITCODE -ne 0) {
    throw "failed to push tag '$Tag' to origin. check permissions or conflicting remote tag."
  }
  Write-Host "tag pushed to origin: $Tag"
} else {
  Write-Host "[1/5] skip tag push (-NoTagPush)"
}

Write-Host "[2/5] building release binary"
cargo build --release | Out-Host

if (-not (Test-Path $binPath)) {
  throw "build output not found: $binPath"
}

$tmpDir = Join-Path $env:TEMP ("prpilot-release-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
  Write-Host "[3/5] packaging $archiveName"
  $stagingDir = Join-Path $tmpDir "staging"
  New-Item -ItemType Directory -Path $stagingDir | Out-Null
  Copy-Item $binPath (Join-Path $stagingDir "prpilot.exe")

  $archivePath = Join-Path (Get-Location) $archiveName
  if (Test-Path $archivePath) { Remove-Item $archivePath -Force }
  Compress-Archive -Path (Join-Path $stagingDir "prpilot.exe") -DestinationPath $archivePath

  $sha256 = (Get-FileHash -Algorithm SHA256 $archivePath).Hash.ToLower()

  Write-Host "[4/5] uploading package -> $packageUrl"
  Invoke-WebRequest -Method Put -Uri $packageUrl -Headers @{"PRIVATE-TOKEN" = $Token} -InFile $archivePath | Out-Null

  Write-Host "uploaded: $archiveName"
  Write-Host "sha256 : $sha256"

  if (-not $NoRelease) {
    Write-Host "[5/5] creating/updating release metadata"
    $releaseEndpoint = "$($GitLabUrl.TrimEnd('/'))/api/v4/projects/$ProjectId/releases"
    $createPayload = @{
      name = "$PackageName $Tag"
      tag_name = $Tag
      description = "$PackageName $Tag`n`n- os: windows`n- arch: $arch`n- sha256: $sha256"
      assets = @{
        links = @(
          @{
            name = $archiveName
            url = $packageUrl
            link_type = "package"
          }
        )
      }
    } | ConvertTo-Json -Depth 8

    $updatePayload = @{
      name = "$PackageName $Tag"
      description = "$PackageName $Tag`n`n- os: windows`n- arch: $arch`n- sha256: $sha256"
      assets = @{
        links = @(
          @{
            name = $archiveName
            url = $packageUrl
            link_type = "package"
          }
        )
      }
    } | ConvertTo-Json -Depth 8

    try {
      Invoke-RestMethod -Method Post -Uri $releaseEndpoint -Headers @{"PRIVATE-TOKEN" = $Token; "Content-Type" = "application/json"} -Body $createPayload | Out-Null
      Write-Host "release created: $Tag"
    } catch {
      try {
        Invoke-RestMethod -Method Put -Uri "$releaseEndpoint/$Tag" -Headers @{"PRIVATE-TOKEN" = $Token; "Content-Type" = "application/json"} -Body $updatePayload | Out-Null
        Write-Host "release updated: $Tag"
      } catch {
        Write-Warning "release create/update failed. package upload succeeded."
      }
    }
  } else {
    Write-Host "[5/5] skip release create/update (-NoRelease)"
  }

  Write-Host ""
  Write-Host "Done."
  Write-Host "Install (Windows) example:"
  Write-Host "  .\\scripts\\install-gitlab.ps1 -ProjectId $ProjectId -Tag $Tag -GitLabUrl $GitLabUrl -Token <TOKEN>"
}
finally {
  Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
}

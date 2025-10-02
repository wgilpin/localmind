Write-Host "Starting Vite dev server..." -ForegroundColor Cyan
$viteProcess = Start-Process npm -ArgumentList "run", "dev" -PassThru -WindowStyle Normal

Start-Sleep -Seconds 3

Write-Host "Starting Tauri..." -ForegroundColor Cyan
try {
    cargo run
} finally {
    Write-Host "`nShutting down Vite dev server..." -ForegroundColor Yellow
    if (!$viteProcess.HasExited) {
        Stop-Process -Id $viteProcess.Id -Force
        Write-Host "Vite dev server stopped." -ForegroundColor Green
    }
}

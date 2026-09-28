param(
    [Parameter(Mandatory=$true)][string]$Source,
    [Parameter(Mandatory=$true)][string]$Destination
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$sourcePath = [IO.Path]::GetFullPath($Source)
$destinationPath = [IO.Path]::GetFullPath($Destination)
if ($sourcePath -eq $destinationPath) { throw 'Source artwork must not be overwritten.' }
# Match createImageCache in desktop/src/wave.js (640 * 1.35).
$width = 864
$image = [Drawing.Image]::FromFile($sourcePath, $true)
try {
    $height = [int][Math]::Ceiling($width * ($image.Height / [double]$image.Width))
    $bitmap = [Drawing.Bitmap]::new($width, $height, [Drawing.Imaging.PixelFormat]::Format24bppRgb)
    try {
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        $attributes = [Drawing.Imaging.ImageAttributes]::new()
        try {
            $graphics.CompositingQuality = [Drawing.Drawing2D.CompositingQuality]::HighQuality
            $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $attributes.SetWrapMode([Drawing.Drawing2D.WrapMode]::TileFlipXY)
            $graphics.DrawImage($image, [Drawing.Rectangle]::new(0,0,$width,$height), 0,0,$image.Width,$image.Height,[Drawing.GraphicsUnit]::Pixel,$attributes)
        } finally { $attributes.Dispose(); $graphics.Dispose() }
        $codec = [Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object MimeType -eq 'image/jpeg'
        $parameters = [Drawing.Imaging.EncoderParameters]::new(1)
        try {
            $parameters.Param[0] = [Drawing.Imaging.EncoderParameter]::new([Drawing.Imaging.Encoder]::Quality, [long]97)
            [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($destinationPath)) | Out-Null
            $bitmap.Save($destinationPath, $codec, $parameters)
        } finally { $parameters.Dispose() }
        $bytes = (Get-Item -LiteralPath $destinationPath).Length
        if ($bytes -ge (Get-Item -LiteralPath $sourcePath).Length) { throw 'Packaged background is not smaller than the source.' }
        Write-Output "Packaged background: ${width}x${height}, $bytes bytes (JPEG quality 97)"
    } finally { $bitmap.Dispose() }
} finally { $image.Dispose() }


$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$interfaces = @{}
Get-NetIPInterface -AddressFamily IPv4 |
  Where-Object ConnectionState -eq "Connected" |
  ForEach-Object { $interfaces[$_.InterfaceIndex] = $_ }
$route = Get-NetRoute -AddressFamily IPv4 -DestinationPrefix "0.0.0.0/0" |
  Where-Object { $interfaces.ContainsKey($_.InterfaceIndex) } |
  Sort-Object @{ Expression = {
    $_.RouteMetric + $interfaces[$_.InterfaceIndex].InterfaceMetric
  } }, InterfaceIndex |
  Select-Object -First 1

$validIpv4 = $false
$defaultRoute = $null -ne $route
$gateway = $false
$interfaceIndex = if ($defaultRoute) { [int]$route.InterfaceIndex } else { 0 }
if ($defaultRoute) {
  $addresses = @(Get-NetIPAddress -AddressFamily IPv4 -InterfaceIndex $route.InterfaceIndex -ErrorAction SilentlyContinue)
  $validIpv4 = @(
    $addresses | Where-Object {
      $_.IPAddress -notmatch "^169\.254\." -and
      $_.IPAddress -ne "0.0.0.0"
    }
  ).Count -gt 0
  $gateway = -not [string]::IsNullOrWhiteSpace([string]$route.NextHop) -and
    [string]$route.NextHop -ne "0.0.0.0"
}

$dns = $false
foreach ($hostName in @("www.microsoft.com", "api.github.com")) {
  try {
    $dns = $null -ne (
      Resolve-DnsName $hostName -DnsOnly -QuickTimeout -ErrorAction Stop |
        Select-Object -First 1
    )
    if ($dns) { break }
  } catch {}
}

$https = $false
foreach ($endpoint in @("https://www.microsoft.com", "https://api.github.com")) {
  try {
    Invoke-WebRequest -Uri $endpoint -Method Head -UseBasicParsing -TimeoutSec 5 -ErrorAction Stop |
      Out-Null
    $https = $true
    break
  } catch {}
}

[pscustomobject]@{
  validIpv4 = $validIpv4
  defaultRoute = $defaultRoute
  gateway = $gateway
  dns = $dns
  https = $https
  interfaceIndex = $interfaceIndex
  nextHop = if ($defaultRoute) { [string]$route.NextHop } else { $null }
} | ConvertTo-Json -Compress

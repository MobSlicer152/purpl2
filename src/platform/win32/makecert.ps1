# Generated entirely by ChatGPT

param (
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$ManifestFilePath,

    [Parameter(Mandatory = $true, Position = 1)]
    [string]$PfxFilePath,

    [Parameter(Mandatory = $true, Position = 2)]
    [string]$CertPassword
)

try {
    # Read the Publisher attribute from the Identity tag in the AppxManifest.xml file
    $Manifest = [xml](Get-Content -Path $ManifestFilePath)
    $Publisher = $Manifest.Package.Identity.Publisher

    # Generate a self-signed certificate
    $CertSubject = "$Publisher"
    $Cert = New-SelfSignedCertificate -Type Custom -Subject $CertSubject -KeyExportPolicy Exportable -KeySpec Signature -KeyLength 2048 -NotAfter (Get-Date).AddYears(1)

    # Prompt the user to import the certificate into the Trusted Root Certification Authorities store
    $ImportRootCert = Read-Host -Prompt "Do you want to import the certificate into the Trusted Root Certification Authorities store? (Y/N)"
    if ($ImportRootCert -eq "Y" -or $ImportRootCert -eq "y") {
        $CertThumbprint = $Cert.Thumbprint
        $RootCertStore = Get-Item -Path "Cert:\LocalMachine\Root"
        $RootCertStore.Open("ReadWrite")
        $RootCertStore.Add($Cert)
        $RootCertStore.Close()
        Write-Host "Certificate imported into the Trusted Root Certification Authorities store!"
    }

    # Export the certificate as a PFX file
    Export-PfxCertificate -Cert $Cert -FilePath $PfxFilePath -Password (ConvertTo-SecureString -String $CertPassword -Force -AsPlainText)

    Write-Host "Certificate generated and exported successfully!"
    Write-Host "Certificate Subject: $CertSubject"
    Write-Host "Certificate Path: $PfxFilePath"

    # Remove the certificate from the certificate store if it was not imported into the Trusted Root Certification Authorities store
    if (!($ImportRootCert -eq "Y" -or $ImportRootCert -eq "y")) {
        $CertThumbprint = $Cert.Thumbprint
        Remove-Item -Path "Cert:\CurrentUser\My\$CertThumbprint" -Force
        Write-Host "Certificate removed from the certificate store!"
    }
}
catch {
    Write-Host "An error occurred while generating the certificate and exporting as PFX:"
    Write-Host $_.Exception.Message
}

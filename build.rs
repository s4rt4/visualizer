fn main() {
    #[cfg(windows)]
    {
        let icon_path = if std::path::Path::new("assets/logo.ico").exists() {
            Some("assets/logo.ico")
        } else if std::path::Path::new("logo.ico").exists() {
            Some("logo.ico")
        } else if std::path::Path::new("assets/icon.ico").exists() {
            Some("assets/icon.ico")
        } else {
            None
        };
        if let Some(icon_path) = icon_path {
            let mut res = winres::WindowsResource::new();
            res.set_icon(icon_path);
            res.set_manifest(
                r#"
                <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
                    <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
                        <security>
                            <requestedPrivileges>
                                <requestedExecutionLevel level="asInvoker" uiAccess="false" />
                            </requestedPrivileges>
                        </security>
                    </trustInfo>
                    <application>
                        <windowsSettings>
                            <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/PM</dpiAware>
                        </windowsSettings>
                    </application>
                </assembly>
                "#,
            );
            res.compile().expect("failed to compile Windows resources");
        }
    }
}

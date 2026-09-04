DELETE FROM mail_context
WHERE template = 'desktop-start'
    AND section IN ('label_mobile', 'scan_qr', 'mobile_install', 'download_google', 'download_apple');

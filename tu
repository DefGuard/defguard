[1m[0m[38;5;11m───────────────────────────────────────────────────────────────────────────────[0m
[38;5;11mmodified: web/src/pages/settings/SettingsIndexPage/tabs/SettingsLicenseTab/components/SettingsLicenseExpiredNotice/SettingsLicenseExpiredNotice.tsx
[38;5;11m───────────────────────────────────────────────────────────────────────────────[0m
[36m[1m[38;5;13m@ web/src/pages/settings/SettingsIndexPage/tabs/SettingsLicenseTab/components/SettingsLicenseExpiredNotice/SettingsLicenseExpiredNotice.tsx:36 @[0m[1m[38;5;146m export const SettingsLicenseExpiredNotice = ({ licenseInfo, state }: Props) => {[0m
          <img src={expiredImage} alt="" />[m
        </div>[m
        <div className="content-track">[m
[31m          <p className="title">{m.settings_license_expired_notice_title()}</p>[m
[31m          <p className="description">{description}</p>[m
[32m[m[32m          <div className="text-track">[m
[32m[m[32m            <p className="title">{m.settings_license_expired_notice_title()}</p>[m
[32m[m[32m            <p className="description">{description}</p>[m
[32m[m[32m          </div>[m
          <a[m
            href={externalLink.defguard.pricing}[m
            rel="noreferrer noopener"[m
[1m[0m[38;5;11m───────────────────────────────────────────────────────────────────────────────[0m
[38;5;11mmodified: web/src/pages/settings/SettingsIndexPage/tabs/SettingsLicenseTab/style.scss
[38;5;11m───────────────────────────────────────────────────────────────────────────────[0m
[36m[1m[38;5;13m@ web/src/pages/settings/SettingsIndexPage/tabs/SettingsLicenseTab/style.scss:72 @[0m[0m
}[m
[m
#license-expired-notice {[m
[32m[m[32m  padding: var(--spacing-lg) var(--spacing-xl);[m
[7m[32m [m
  .notice-track {[m
    display: grid;[m
    grid-template-columns: 68px 1fr;[m
[36m[1m[38;5;13m@ web/src/pages/settings/SettingsIndexPage/tabs/SettingsLicenseTab/style.scss:92 @[0m[0m
      align-items: flex-start;[m
      gap: var(--spacing-md);[m
[m
[32m[m[32m      .text-track {[m
[32m[m[32m        display: flex;[m
[32m[m[32m        flex-flow: column;[m
[32m[m[32m        align-items: flex-start;[m
[32m[m[32m        gap: var(--spacing-xs);[m
[32m[m[32m      }[m
[7m[32m [m
      .title {[m
        color: var(--fg-critical);[m
        font: var(--t-title-h5);[m

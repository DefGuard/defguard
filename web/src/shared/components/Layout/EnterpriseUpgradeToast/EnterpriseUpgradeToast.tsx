import './style.scss';

import { useCallback } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Badge } from '../../../defguard-ui/components/Layout/Badge/Badge';
import { BadgeStyleVariant } from '../../../defguard-ui/components/Layout/Badge/types';
import { ToastOptions } from '../../../defguard-ui/components/Layout/ToastManager/Toast/types';
import { useToastsStore } from '../../../defguard-ui/hooks/toasts/useToastStore';
import SvgIconX from '../../svg/IconX';

export const EnterpriseUpgradeToast = ({ id }: ToastOptions) => {
  const removeToast = useToastsStore((s) => s.removeToast);
  const { LL } = useI18nContext();

  const closeToast = useCallback(() => {
    removeToast(id);
  }, [id, removeToast]);

  const handleDismiss = () => {
    closeToast();
  };

  return (
    <div className="enterprise-upgrade-toaster">
      <div className="top">
        <div className="heading">
          <Badge
            styleVariant={BadgeStyleVariant.PRIMARY}
            icon={
              <svg
                width="8"
                height="10"
                viewBox="0 0 8 10"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
              >
                <path
                  d="M4.75294 0.891119C4.75294 0.553623 4.47935 0.280029 4.14185 0.280029C3.80436 0.280029 3.53076 0.553623 3.53076 0.891119V9.38893C3.53076 9.72642 3.80436 10 4.14185 10C4.47935 10 4.75294 9.72642 4.75294 9.38893V0.891119Z"
                  fill="white"
                />
                <path
                  d="M4.54343 1.29638C4.78208 1.05773 4.78208 0.670812 4.54343 0.432167C4.30479 0.193521 3.91787 0.193521 3.67922 0.432167L1.51869 2.59269C1.28005 2.83134 1.28005 3.21826 1.5187 3.45691C1.75734 3.69555 2.14426 3.69555 2.38291 3.45691L4.54343 1.29638Z"
                  fill="white"
                />
                <path
                  d="M4.5739 0.432152C4.33526 0.193507 3.94834 0.193507 3.70969 0.432152C3.47105 0.670798 3.47105 1.05772 3.70969 1.29636L5.87022 3.45689C6.10887 3.69554 6.49579 3.69554 6.73443 3.45689C6.97308 3.21825 6.97308 2.83132 6.73443 2.59268L4.5739 0.432152Z"
                  fill="white"
                />
              </svg>
            }
            className="toaster-badge"
          />
          <p>{LL.modals.enterpriseUpgradeToaster.title()}</p>
        </div>
        <button className="dismiss" onClick={handleDismiss}>
          <SvgIconX width={14} height={14} />
        </button>
      </div>
      <div className="bottom">
        <p>{LL.modals.enterpriseUpgradeToaster.message()}</p>
        <div className="upgrade-link-container">
          <a
            href="https://defguard.net/pricing/"
            target="_blank"
            rel="noreferrer noopener"
            className="upgrade-link"
          >
            {LL.modals.enterpriseUpgradeToaster.link()}
          </a>
        </div>
      </div>
    </div>
  );
};

import clsx from 'clsx';
import { m } from '../../../../../../../../../paraglide/messages';
import { Badge } from '../../../../../../../../../shared/defguard-ui/components/Badge/Badge';
import type { IconKindValue } from '../../../../../../../../../shared/defguard-ui/components/Icon/icon-types';
import type { MenuItemsGroup } from '../../../../../../../../../shared/defguard-ui/components/Menu/types';
import { isPresent } from '../../../../../../../../../shared/defguard-ui/utils/isPresent';
import { AuthCardRow } from '../AuthCardRow/AuthCardRow';

type Props = {
  icon: IconKindValue;
  title: string;
  enabled: boolean;
  isDefault: boolean;
  menu?: MenuItemsGroup | null;
  testId?: string;
  smtpDisabled?: boolean;
};

export const FactorRow = ({
  icon,
  title,
  enabled,
  isDefault,
  menu,
  testId,
  smtpDisabled = false,
}: Props) => {
  return (
    <AuthCardRow
      icon={icon}
      title={title}
      menu={isPresent(menu) && !smtpDisabled ? { itemGroups: [menu] } : undefined}
      testId={testId}
    >
      {smtpDisabled ? (
        <>
          <p className="status disabled">{m.state_not_configured()}</p>
          <span className="separator" />
          <p>{m.state_smtp_not_configured()}</p>
        </>
      ) : (
        <>
          <p className={clsx('status', { disabled: !enabled })}>
            {enabled ? m.state_enabled() : m.state_disabled()}
          </p>
          <span className="separator" />
          <p>{m.profile_auth_card_passkey_usage_web()}</p>
          <span className="separator" />
          <p>{m.profile_auth_card_passkey_usage_vpn()}</p>
          {isDefault && (
            <Badge
              variant="success"
              size="compact"
              text={m.profile_auth_card_default_method()}
            />
          )}
        </>
      )}
    </AuthCardRow>
  );
};

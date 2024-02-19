import classNames from 'classnames';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { ActivityIcon } from '../../../../../../../../shared/defguard-ui/components/icons/ActivityIcon/ActivityIcon';
import { ActivityIconVariant } from '../../../../../../../../shared/defguard-ui/components/icons/ActivityIcon/types';
import { SelectRow } from '../../../../../../../../shared/defguard-ui/components/Layout/SelectRow/SelectRow';

type Props = {
  selected: boolean;
  onClick: () => void;
  name: string;
  available?: boolean;
};

export const ProvisionerRow = ({ selected, onClick, name, available }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.userPage.authenticationKeys.addModal.yubikeyForm;

  return (
    <SelectRow selected={selected} onClick={onClick} type="radio" highlightActive>
      <p className="name">{name}</p>
      <div
        className={classNames('availability', {
          available: available,
        })}
      >
        <p>
          {available
            ? localLL.selectWorker.available()
            : localLL.selectWorker.unavailable()}
        </p>
        <div className="icon-wrapper">
          {available && <ActivityIcon status={ActivityIconVariant.CONNECTED} />}
          {!available && <ActivityIcon status={ActivityIconVariant.BLANK} />}
        </div>
      </div>
    </SelectRow>
  );
};

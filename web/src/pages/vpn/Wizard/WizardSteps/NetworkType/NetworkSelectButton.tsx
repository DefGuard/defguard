import React from 'react';
import { useTranslation } from 'react-i18next';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import SvgIconCheckmarkWhiteBig from '../../../../../shared/components/svg/IconCheckmarkWhiteBig';

type Active = {
  active: boolean;
  onClick: React.MouseEventHandler<HTMLButtonElement>;
};

const NetworkSelectButton: React.FC<Active> = ({ active, onClick }) => {
  const { t } = useTranslation('en');
  return (
    <Button
      styleVariant={
        active ? ButtonStyleVariant.CONFIRM_SUCCESS : ButtonStyleVariant.PRIMARY
      }
      size={ButtonSize.BIG}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        onClick(e);
      }}
      text={!active ? t('wizard.common.select') : undefined}
      icon={active ? <SvgIconCheckmarkWhiteBig /> : undefined}
      type="button"
    />
  );
};

export default NetworkSelectButton;

import './style.scss';

import { ReactNode } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import { Card } from '../../../../../../shared/components/layout/Card/Card';
import { IconCheckmarkWhite } from '../../../../../../shared/components/svg';

type Props = {
  icon: ReactNode;
  title: string;
  subtitle: string;
  onClick: () => void;
  selected: boolean;
};
export const WizardTypeOptionCard = ({
  icon,
  title,
  subtitle,
  onClick,
  selected,
}: Props) => {
  const { LL } = useI18nContext();
  return (
    <Card className="wizard-network-option">
      <header>
        <p>{title}</p>
      </header>
      <p>{subtitle}</p>
      {icon}
      <Button
        styleVariant={
          selected ? ButtonStyleVariant.CONFIRM_SUCCESS : ButtonStyleVariant.PRIMARY
        }
        icon={selected ? <IconCheckmarkWhite /> : undefined}
        text={!selected ? LL.wizard.common.select() : undefined}
        onClick={onClick}
        size={ButtonSize.BIG}
      />
    </Card>
  );
};

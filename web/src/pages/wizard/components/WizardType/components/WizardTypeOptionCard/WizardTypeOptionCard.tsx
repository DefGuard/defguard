import './style.scss';

import { ReactNode } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../../shared/components/svg/IconCheckmarkWhite';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';

type Props = {
  icon: ReactNode;
  title: string;
  subtitle: string;
  onClick: () => void;
  selected: boolean;
  testId?: string;
};
export const WizardTypeOptionCard = ({
  icon,
  title,
  subtitle,
  onClick,
  selected,
  testId,
}: Props) => {
  const { LL } = useI18nContext();
  return (
    <Card className="wizard-network-option" shaded>
      <header>
        <p>{title}</p>
      </header>
      <p>{subtitle}</p>
      {icon}
      <Button
        styleVariant={selected ? ButtonStyleVariant.SAVE : ButtonStyleVariant.PRIMARY}
        icon={selected ? <IconCheckmarkWhite /> : undefined}
        text={!selected ? LL.wizard.common.select() : undefined}
        onClick={onClick}
        size={ButtonSize.LARGE}
        data-testid={testId}
      />
    </Card>
  );
};

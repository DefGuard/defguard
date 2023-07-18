import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { RenderTranslation } from '../../../../shared/components/i18n/RenderTranslation/RenderTranslation';
import { Button } from '../../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/types';
import { Card } from '../../../../shared/components/layout/Card/Card';
import { IconInfo } from '../../../../shared/components/svg';
import { useWizardStore } from '../../hooks/useWizardStore';

export const WizardWelcome = () => {
  const nextStep = useWizardStore((state) => state.nextStep);
  const { LL } = useI18nContext();
  return (
    <Card id="wizard-welcome" shaded hideMobile>
      <header>
        <p>{LL.wizard.welcome.header()}</p>
      </header>
      <p>
        <RenderTranslation
          translation={LL.wizard.welcome.sub()}
          component={<IconInfo key={0} />}
        />
      </p>
      <Button
        onClick={() => nextStep()}
        size={ButtonSize.LARGE}
        styleVariant={ButtonStyleVariant.PRIMARY}
        text={LL.wizard.welcome.button()}
        data-testid="setup-network"
      />
    </Card>
  );
};

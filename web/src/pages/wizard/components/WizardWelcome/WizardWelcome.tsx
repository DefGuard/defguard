import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { RenderTranslation } from '../../../../shared/components/i18n/RenderTranslation/RenderTranslation';
import { IconInfo } from '../../../../shared/components/svg';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
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
          components={[<IconInfo key={0} />]}
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

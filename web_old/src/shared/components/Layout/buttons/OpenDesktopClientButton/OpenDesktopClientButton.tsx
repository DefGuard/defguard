import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Button } from '../../../../defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../defguard-ui/components/Layout/Button/types';

type Props = {
  url: string;
  token: string;
  customMessage?: string;
};

export const OpenDesktopClientButton = ({ token, url, customMessage }: Props) => {
  const { LL } = useI18nContext();
  const makeUrl = () => {
    return `defguard://addinstance?token=${token}&url=${url}`;
  };

  return (
    <a href={makeUrl()} className="desktop-client-deep-link">
      <Button
        type="button"
        size={ButtonSize.LARGE}
        styleVariant={ButtonStyleVariant.PRIMARY}
        text={customMessage ?? LL.components.openClientDeepLink()}
      />
    </a>
  );
};

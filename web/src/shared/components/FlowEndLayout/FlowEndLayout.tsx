import './style.scss';
import { Button } from '../../defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../defguard-ui/components/Button/types';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { NavLogo } from '../Navigation/assets/NavLogo';
import { FlowEndImage } from './components/FlowEndImage/FlowEndImage';
import type { FlowEndImageVariantValue } from './components/FlowEndImage/types';

type Props = {
  image: FlowEndImageVariantValue;
  title: string;
  subtitle?: string;
  actionProps?: ButtonProps;
};

export const FlowEndLayout = ({ image, title, subtitle, actionProps }: Props) => {
  return (
    <div className="flow-end-layout">
      <div className="top">
        <NavLogo />
      </div>
      <div className="main-track">
        <div className="main">
          <FlowEndImage variant={image} />
          <SizedBox height={ThemeSpacing.Xl3} />
          <h1>{title}</h1>
          {isPresent(subtitle) && <h2>{subtitle}</h2>}
          {isPresent(actionProps) && (
            <>
              <SizedBox height={ThemeSpacing.Xl3} />
              <Button {...actionProps} />
            </>
          )}
        </div>
      </div>
    </div>
  );
};

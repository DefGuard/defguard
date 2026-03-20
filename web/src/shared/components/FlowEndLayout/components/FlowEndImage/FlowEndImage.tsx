import image404 from './assets/404.png';
import appErrorImage from './assets/app_error.png';
import type { FlowEndImageVariantValue } from './types';

interface Props {
  variant: FlowEndImageVariantValue;
}

export const FlowEndImage = ({ variant }: Props) => {
  switch (variant) {
    case '404':
      return <img src={image404} width={100} height={111} />;
    case 'app-error':
      return <img src={appErrorImage} width={108} height={111} />;
  }
};

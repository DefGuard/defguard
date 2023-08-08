import { Card } from '../../../shared/defguard-ui/components/Layout/Card/Card';
import './style.scss';

import TeoniteLogoGif from '/src/shared/images/gif/tnt-built.gif';

export const BuiltByCard = () => {
  return (
    <Card className="built-by">
      <a href="https://www.teonite.com" target="_blank" rel="noreferrer">
        <img src={TeoniteLogoGif} alt="logo" />
      </a>
    </Card>
  );
};

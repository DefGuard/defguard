import './style.scss';

import { Card } from '../../../shared/components/layout/Card/Card';
import TeoniteLogoGif from '../../../shared/images/gif/tnt-built.gif';

export const BuiltByCard = () => {
  return (
    <Card className="built-by">
      <a href="https://www.teonite.com" target="_blank" rel="noreferrer">
        <img src={TeoniteLogoGif} alt="logo" />
      </a>
    </Card>
  );
};

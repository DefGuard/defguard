import './style.scss';

// eslint-disable-next-line import/no-unresolved
import TeoniteLogoGif from '/src/shared/images/gif/tnt-built.gif';

import { Card } from '../../../shared/defguard-ui/components/Layout/Card/Card';

export const BuiltByCard = () => {
  return (
    <Card className="built-by">
      <a href="https://www.teonite.com" target="_blank" rel="noreferrer">
        <img src={TeoniteLogoGif} alt="logo" />
      </a>
    </Card>
  );
};

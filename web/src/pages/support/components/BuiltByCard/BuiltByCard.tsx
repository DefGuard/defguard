import './style.scss';

// eslint-disable-next-line import/no-unresolved
import TeoniteLogoGif from '/src/shared/images/gif/tnt-built.gif';

import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';

export const BuiltByCard = () => {
  return (
    <Card id="built-by-card" shaded bordered>
      <a href="https://www.teonite.com" target="_blank" rel="noreferrer">
        <img src={TeoniteLogoGif} alt="logo" />
      </a>
    </Card>
  );
};

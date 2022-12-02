import './style.scss';

import { Card } from '../../../../shared/components/layout/Card/Card';
import { OpenidClient } from '../../../../shared/types';
import OpenidClientInfo from './OpenidClientInfo';

interface Props {
  client: OpenidClient;
}

const OpenidClientDetail = ({ client }: Props) => {
  return (
    <section id="client-details">
      <header>
        <h2>App Details</h2>
      </header>
      <Card>
        <OpenidClientInfo client={client} />
      </Card>
    </section>
  );
};

export default OpenidClientDetail;

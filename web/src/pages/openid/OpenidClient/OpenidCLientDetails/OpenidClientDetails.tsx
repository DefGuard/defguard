import './style.scss';

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
      <div className="card">
        <OpenidClientInfo client={client} />
      </div>
    </section>
  );
};

export default OpenidClientDetail;

import React from 'react';

import { Label } from '../../../../shared/components/layout/Label/Label';
import { Tag } from '../../../../shared/components/layout/Tag/Tag';
import { OpenidClient } from '../../../../shared/types';
import { titleCase } from '../../../../shared/utils/titleCase';
interface Props {
  client: OpenidClient;
}

const OpenidClientInfo: React.FC<Props> = ({ client }) => {
  return (
    <>
      <div className="row">
        <div className="info">
          <Label>Name</Label>
          <p>{client.name}</p>
        </div>
      </div>
      <div className="row">
        <div className="info">
          <Label>Client id</Label>
          <p>{client.client_id}</p>
        </div>
        <div className="info">
          <Label>Client secret</Label>
          <p>{client.client_secret}</p>
        </div>
      </div>
      <div className="row tags">
        <Label>Redirect urls</Label>
        <div className="tags">
          {client.redirect_uri.map((url) => (
            <Tag disposable={false} text={titleCase(url)} key={url} />
          ))}
        </div>
        <Label>Scopes</Label>
        <div className="tags">
          {client.scope.map((scope) => (
            <Tag disposable={false} text={titleCase(scope)} key={scope} />
          ))}
        </div>
      </div>
    </>
  );
};

export default OpenidClientInfo;

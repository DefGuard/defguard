import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';

//import SvgIconUserListExpanded from '../../../../shared/components/svg/IconNavUsers';
import { DeviceAvatar } from '../../../../shared/components/layout/DeviceAvatar/DeviceAvatar';
import Divider from '../../../../shared/components/layout/Divider/Divider';
import SvgIconCheckmarkGreen from '../../../../shared/components/svg/IconCheckmarkGreen';
import SvgIconDisconnected from '../../../../shared/components/svg/IconDisconnected';
import SvgIconUserList from '../../../../shared/components/svg/IconUserList';
import SvgIconUserListExpanded from '../../../../shared/components/svg/IconUserListExpanded';
import { useNavigationStore } from '../../../../shared/hooks/store/useNavigationStore';
import { OpenidClient } from '../../../../shared/types';
import OpenidClientEditButton from '../OpenidClientsListTable/OpenidClientEditButton';

interface Props {
  client: OpenidClient;
}

const OpenidClientListItem: React.FC<Props> = ({ client }) => {
  const [expanded, setExpanded] = useState(false);
  const navigate = useNavigate();
  const setNavigationOpenidClient = useNavigationStore(
    (state) => state.setNavigationOpenidClient
  );

  const navigateToOpenidClient = () => {
    setNavigationOpenidClient(client);
    navigate(`/admin/openid/${client.client_id}`, { replace: true });
  };

  return (
    <div className="client-container">
      <section className="top">
        <div
          className="collapse-icon-container"
          onClick={() => setExpanded((state) => !state)}
        >
          {expanded ? <SvgIconUserListExpanded /> : <SvgIconUserList />}
        </div>
        <DeviceAvatar active={client.enabled} />
        <p className="name" onClick={navigateToOpenidClient}>
          {client.name}
        </p>
        <OpenidClientEditButton client={client} />
      </section>
      {expanded ? (
        <>
          <Divider />
          <section className="client-details-collapse">
            <div>
              <label>Status:</label>
              <div className="status">
                {client.enabled ? (
                  <SvgIconCheckmarkGreen />
                ) : (
                  <SvgIconDisconnected />
                )}
                <p>{client.enabled ? 'Enabled' : 'Disabled'}</p>
              </div>
            </div>
          </section>
        </>
      ) : null}
    </div>
  );
};

export default OpenidClientListItem;

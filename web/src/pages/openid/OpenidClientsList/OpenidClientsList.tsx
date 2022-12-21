import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import { useState } from 'react';
import { useMemo } from 'react';
import { useNavigate } from 'react-router';
import Select from 'react-select';
import useBreakpoint from 'use-breakpoint';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import NoData from '../../../shared/components/layout/NoData/NoData';
import { Search } from '../../../shared/components/layout/Search/Search';
import SvgIconPlusWhite from '../../../shared/components/svg/IconPlusWhite';
import { deviceBreakpoints } from '../../../shared/constants';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import { useNavigationStore } from '../../../shared/hooks/store/useNavigationStore';
import useApi from '../../../shared/hooks/useApi';
import { QueryKeys } from '../../../shared/queries';
import { OpenidClient } from '../../../shared/types';
import { standardVariants } from '../../../shared/variants';
import AddOpenidClientModal from './AddOpenidClientModal/AddOpenidClientModal';

export const OpenidClientsList = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const navigate = useNavigate();
  const {
    openid: { getOpenidClients },
    license: { getLicense },
  } = useApi();

  const { data: license } = useQuery([QueryKeys.FETCH_LICENSE], getLicense);

  const hasAccess = useMemo(() => {
    return license?.openid || license?.enterprise;
  }, [license]);

  const { data: clients, isLoading } = useQuery(
    [QueryKeys.FETCH_CLIENTS],
    getOpenidClients,
    { enabled: hasAccess }
  );
  const [clientsSearchValue, setClientsSearchValue] = useState('');
  const setNavigationOpenidClient = useNavigationStore(
    (state) => state.setNavigationOpenidClient
  );
  const setOpenidClientAddModalState = useModalStore(
    (state) => state.setAddOpenidClientModal
  );
  const navigateToClient = (client: OpenidClient) => {
    setNavigationOpenidClient(client);
    navigate(`${client.client_id}`);
  };

  return (
    <section id="clients-list">
      <motion.header
        variants={standardVariants}
        initial="hidden"
        animate="show"
      >
        <h1>OpenID Apps</h1>
        {breakpoint !== 'mobile' ? (
          <Search
            placeholder="Find app"
            className="clients-search"
            initialValue={clientsSearchValue}
            onChange={(value) => setClientsSearchValue(value)}
          />
        ) : null}
      </motion.header>
      <motion.section
        className="actions"
        variants={standardVariants}
        initial="hidden"
        animate="show"
      >
        <div className="clients-count">
          <span>All apps</span>
          <div className="count" data-test="clients-count">
            <span>{clients && clients.length > 0 ? clients.length : 0}</span>
          </div>
        </div>
        <div className="table-controls">
          {breakpoint !== 'mobile' ? (
            <Select
              placeholder="All apps"
              options={[{ value: 'all', label: 'All apps' }]}
              className="custom-select"
              classNamePrefix="rs"
              isDisabled={!hasAccess}
            />
          ) : null}
          <Button
            className="add-client"
            onClick={() => setOpenidClientAddModalState({ visible: true })}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={<SvgIconPlusWhite />}
            text="Add new"
            disabled={!hasAccess}
          />
        </div>
        {breakpoint === 'mobile' ? (
          <Search
            placeholder="Find apps"
            className="clients-search"
            initialValue={clientsSearchValue}
            onChange={(value) => setClientsSearchValue(value)}
          />
        ) : null}
      </motion.section>

      {!hasAccess ? (
        <NoData customMessage="You don't have a license for this feature" />
      ) : (
        clients &&
        clients.length > 0 &&
        !isLoading && <section className="clients-table"></section>
      )}
      <AddOpenidClientModal />
    </section>
  );
};

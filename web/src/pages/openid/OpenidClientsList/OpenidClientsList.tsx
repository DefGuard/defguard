import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import { orderBy } from 'lodash-es';
import { useState } from 'react';
import { useMemo } from 'react';
import { useNavigate } from 'react-router';
import Select from 'react-select';
import { Column } from 'react-table';
import useBreakpoint from 'use-breakpoint';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import { DeviceAvatar } from '../../../shared/components/layout/DeviceAvatar/DeviceAvatar';
import NoData from '../../../shared/components/layout/NoData/NoData';
import Search from '../../../shared/components/layout/Search/Search';
import { IconDeactivated } from '../../../shared/components/svg';
import SvgIconCheckmarkGreen from '../../../shared/components/svg/IconCheckmarkGreen';
import SvgIconPlusWhite from '../../../shared/components/svg/IconPlusWhite';
import { deviceBreakpoints } from '../../../shared/constants';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import { useNavigationStore } from '../../../shared/hooks/store/useNavigationStore';
import useApi from '../../../shared/hooks/useApi';
import { QueryKeys } from '../../../shared/queries';
import { OpenidClient } from '../../../shared/types';
import { standardVariants } from '../../../shared/variants';
import AddOpenidClientModal from './AddOpenidClientModal/AddOpenidClientModal';
import OpenidClientsListMobile from './OpenidClientsListMobile/OpenidClientsListMobile';
import OpenidClientsListTable from './OpenidClientsListTable/OpenidClientsListTable';

const OpenidClientsList = () => {
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
  const tableColumns: Column<OpenidClient>[] = useMemo(
    () => [
      {
        Header: 'name',
        accessor: 'name',
        Cell: ({ row }) => {
          return (
            <div className="client-name">
              <DeviceAvatar active={row.original.enabled} />
              <p
                className="name"
                onClick={() => navigateToClient(row.original)}
              >
                {row.original.name}
              </p>
            </div>
          );
        },
      },
      {
        Header: 'Status',
        accessor: 'enabled',
        Cell: (cell) => (
          <div className="status">
            {cell.value ? <SvgIconCheckmarkGreen /> : <IconDeactivated />}
            <span className={cell.value ? 'active' : undefined}>
              {cell.value ? 'Enabled' : 'Disabled'}
            </span>
          </div>
        ),
      },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    []
  );

  const filteredClients = useMemo(() => {
    if (!clients || (clients && !clients.length)) {
      return [];
    }
    let searched: OpenidClient[] = [];
    if (clients) {
      searched = clients.filter((client) =>
        client.name
          .toLocaleLowerCase()
          .includes(clientsSearchValue.toLocaleLowerCase())
      );
    }
    if (searched.length) {
      return orderBy(searched, ['name'], ['asc']);
    }
    return searched;
  }, [clients, clientsSearchValue]);

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
            disabled={!hasAccess}
            placeholder="Find app"
            className="clients-search"
            value={clientsSearchValue}
            onChange={(e) => setClientsSearchValue(e.target.value)}
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
            value={clientsSearchValue}
            onChange={(e) => setClientsSearchValue(e.target.value)}
          />
        ) : null}
      </motion.section>

      {!hasAccess ? (
        <NoData customMessage="You don't have a license for this feature" />
      ) : (
        clients &&
        clients.length > 0 &&
        !isLoading &&
        (breakpoint === 'mobile' ? (
          <OpenidClientsListMobile clients={filteredClients} />
        ) : (
          <section className="clients-table">
            <OpenidClientsListTable
              data={filteredClients}
              columns={tableColumns}
            />
          </section>
        ))
      )}
      <AddOpenidClientModal />
    </section>
  );
};

export default OpenidClientsList;

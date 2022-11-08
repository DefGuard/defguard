import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import { clone, orderBy } from 'lodash-es';
import React, { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { Column } from 'react-table';
import useBreakpoint from 'use-breakpoint';

import NoData from '../../shared/components/layout/NoData/NoData';
import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import Search from '../../shared/components/layout/Search/Search';
import { IconDeactivated } from '../../shared/components/svg';
import SvgIconCheckmarkGreen from '../../shared/components/svg/IconCheckmarkGreen';
import { deviceBreakpoints } from '../../shared/constants';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { Provisioner } from '../../shared/types';
import { standardVariants } from '../../shared/variants';
import ProvisionersList from './ProvisionersList/ProvisionersList';
import ProvisionersTable from './ProvisionersTable/ProvisionersTable';
import ProvisioningStationSetup from './ProvisioningStationSetup';

const ProvisionersPage: React.FC = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const [searchValue, setSearchValue] = useState<string>('');
  const [filteredProvisioners, setFilteredProvisioners] = useState<
    Provisioner[]
  >([]);
  const {
    provisioning: { getWorkers },
    license: { getLicense },
  } = useApi();

  const { data: license } = useQuery([QueryKeys.FETCH_LICENSE], getLicense);
  const settings = useAppStore((state) => state.settings);
  const navigate = useNavigate();

  if (!settings?.worker_enabled) navigate('/');

  const hasAccess = useMemo(() => {
    return license?.enterprise || license?.worker;
  }, [license]);

  const { data: provisioners } = useQuery(
    [QueryKeys.FETCH_WORKERS],
    getWorkers,
    { enabled: hasAccess }
  );

  const tableColumns: Column<Provisioner>[] = useMemo(
    (): Column<Provisioner>[] => [
      {
        Header: 'Name',
        accessor: 'id',
        Cell: ({ cell }) => {
          return (
            <div className="provisioner-id">
              {/* <DeviceAvatar active={row.original.connected} /> */}
              <span>{cell.value}</span>
            </div>
          );
        },
      },
      {
        Header: 'Status',
        accessor: 'connected',
        Cell: ({ cell }) => {
          return (
            <div className="connection-status">
              {!cell.value ? <SvgIconCheckmarkGreen /> : <IconDeactivated />}
              <span className={cell.value ? 'active' : undefined}>
                {/* {cell.value ? 'Available' : 'Unavailable'} */}
              </span>
            </div>
          );
        },
      },
      {
        Header: 'IP address',
        accessor: 'ip',
        Cell: ({ row, cell }) => {
          return (
            <span
              className={
                row.original.connected ? 'ip-address active' : 'ip-address'
              }
            >
              {cell.value}
            </span>
          );
        },
      },
    ],
    []
  );

  useEffect(() => {
    if (provisioners) {
      const c = clone(provisioners);
      if (searchValue && searchValue.length) {
        const filtered = c.filter((provisioner) =>
          provisioner.id.toLowerCase().includes(searchValue.toLowerCase())
        );
        const ordered = orderBy(filtered, ['id'], ['desc']);
        setFilteredProvisioners(ordered);
      } else {
        setFilteredProvisioners(provisioners);
      }
    } else {
      setFilteredProvisioners([]);
    }
  }, [provisioners, searchValue]);

  return (
    <PageContainer className="provisioners-page">
      {breakpoint !== 'mobile' ? (
        <motion.header
          initial="hidden"
          animate="show"
          variants={standardVariants}
        >
          <h1>Provisioners</h1>
          <Search
            placeholder="Find"
            value={searchValue}
            onChange={(event) => setSearchValue(event.target.value)}
          />
        </motion.header>
      ) : null}
      <div className="provisioning-container">
        <div className="column">
          <motion.div
            variants={standardVariants}
            initial="hidden"
            animate="show"
            className="actions"
          >
            <div className="provisioners-count">
              <span>All provisioners</span>
              <div className="count">
                <span>{provisioners?.length ?? 0}</span>
              </div>
            </div>
          </motion.div>
          {breakpoint === 'mobile' ? (
            <Search
              containerMotionProps={{
                variants: standardVariants,
                initial: 'hidden',
                animate: 'show',
              }}
              placeholder="Find"
              value={searchValue}
              onChange={(event) => setSearchValue(event.target.value)}
            />
          ) : null}

          {hasAccess ? (
            filteredProvisioners && filteredProvisioners.length ? (
              breakpoint === 'mobile' || breakpoint === 'tablet' ? (
                <ProvisionersList provisioners={filteredProvisioners} />
              ) : (
                <ProvisionersTable
                  columns={tableColumns}
                  data={filteredProvisioners}
                />
              )
            ) : (
              <NoData customMessage="Currently there are no YubiKey stations registered" />
            )
          ) : (
            <NoData customMessage="No license for this feature" />
          )}
        </div>
        <ProvisioningStationSetup hasAccess={hasAccess} />
      </div>
    </PageContainer>
  );
};

export default ProvisionersPage;

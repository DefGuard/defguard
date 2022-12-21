import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import { clone, orderBy } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import useBreakpoint from 'use-breakpoint';

import NoData from '../../shared/components/layout/NoData/NoData';
import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { Search } from '../../shared/components/layout/Search/Search';
import { deviceBreakpoints } from '../../shared/constants';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { Provisioner } from '../../shared/types';
import { standardVariants } from '../../shared/variants';
import ProvisionersList from './ProvisionersList/ProvisionersList';
import ProvisioningStationSetup from './ProvisioningStationSetup';

const ProvisionersPage = () => {
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

  const hasAccess = useMemo(() => {
    return license?.enterprise || license?.worker;
  }, [license]);

  const { data: provisioners } = useQuery(
    [QueryKeys.FETCH_WORKERS],
    getWorkers,
    { enabled: hasAccess }
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
            placeholder="Find provisioners"
            initialValue={searchValue}
            onChange={(value) => setSearchValue(value)}
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
          {breakpoint !== 'desktop' ? (
            <Search
              placeholder="Find provisioners"
              initialValue={searchValue}
              onChange={(value) => setSearchValue(value)}
            />
          ) : null}

          {hasAccess &&
            filteredProvisioners &&
            filteredProvisioners.length > 0 && (
              <ProvisionersList provisioners={filteredProvisioners} />
            )}
          {(hasAccess && !filteredProvisioners) ||
          filteredProvisioners.length === 0 ? (
            <NoData customMessage="No provisioners found" />
          ) : null}
          {!hasAccess && <NoData customMessage="No license for this feature" />}
        </div>
        <ProvisioningStationSetup hasAccess={hasAccess} />
      </div>
    </PageContainer>
  );
};

export default ProvisionersPage;

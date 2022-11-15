import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import { clone, orderBy } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import { Column } from 'react-table';
import useBreakpoint from 'use-breakpoint';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/components/layout/Button/Button';
import { DeviceAvatar } from '../../shared/components/layout/DeviceAvatar/DeviceAvatar';
import NoData from '../../shared/components/layout/NoData/NoData';
import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import Search from '../../shared/components/layout/Search/Search';
import { IconDeactivated } from '../../shared/components/svg';
import SvgIconCheckmarkGreen from '../../shared/components/svg/IconCheckmarkGreen';
import SvgIconPlusWhite from '../../shared/components/svg/IconPlusWhite';
import { deviceBreakpoints } from '../../shared/constants';
import { useModalStore } from '../../shared/hooks/store/useModalStore';
import useApi from '../../shared/hooks/useApi';
import { patternBaseUrl } from '../../shared/patterns';
import { QueryKeys } from '../../shared/queries';
import { Webhook } from '../../shared/types';
import { standardVariants } from '../../shared/variants';
import AddWebhookModal from './modals/AddWebhookModal/AddWebhookModal';
import WebhooksList from './WebhooksList/WebhooksList';
import WebhooksTable from './WebhooksTable/WebhooksTable';

const WebhooksPage = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const [searchValue, setSearchValue] = useState<string>('');
  const [filteredWebhooks, setFilteredWebhooks] = useState<Webhook[]>([]);
  const setWebhookAddModalState = useModalStore(
    (state) => state.setAddWebhookModal
  );

  const {
    webhook: { getWebhooks },
  } = useApi();

  const { data: webhooks } = useQuery([QueryKeys.FETCH_WEBHOOKS], getWebhooks);

  const tableColumns: Column<Webhook>[] = useMemo(
    (): Column<Webhook>[] => [
      {
        Header: 'Url',
        accessor: 'url',
        Cell: ({ row, cell }) => {
          const match = cell.value.match(patternBaseUrl);
          return (
            <div className="webhook-url">
              <DeviceAvatar active={row.original.enabled} />
              <span>{match ? match[1] : 'Pattern not found'}</span>
            </div>
          );
        },
      },
      {
        Header: 'Description',
        accessor: 'description',
        Cell: ({ cell }) => {
          return (
            <div className="webhook-description">
              <span>{cell.value}</span>
            </div>
          );
        },
      },
      {
        Header: 'Status',
        accessor: 'enabled',
        Cell: ({ cell }) => {
          return (
            <div className="connection-status">
              {cell.value ? <SvgIconCheckmarkGreen /> : <IconDeactivated />}
              <span className={cell.value ? 'active' : undefined}>
                {cell.value ? 'Enabled' : 'Disabled'}
              </span>
            </div>
          );
        },
      },
    ],
    []
  );

  useEffect(() => {
    if (webhooks) {
      const c = clone(webhooks);
      if (searchValue && searchValue.length) {
        const filtered = c.filter((webhook) =>
          webhook.url.toLowerCase().includes(searchValue.toLowerCase())
        );
        const ordered = orderBy(filtered, ['id'], ['desc']);
        setFilteredWebhooks(ordered);
      } else {
        setFilteredWebhooks(webhooks);
      }
    } else {
      setFilteredWebhooks([]);
    }
  }, [webhooks, searchValue]);

  return (
    <PageContainer className="webhooks-page">
      {breakpoint !== 'mobile' ? (
        <motion.header
          initial="hidden"
          animate="show"
          variants={standardVariants}
        >
          <h1>Webhooks</h1>
          <Search
            placeholder="Find"
            value={searchValue}
            onChange={(event) => setSearchValue(event.target.value)}
          />
        </motion.header>
      ) : null}
      <motion.section
        variants={standardVariants}
        initial="hidden"
        animate="show"
        className="actions"
      >
        <div className="webhooks-count">
          <span>All webhooks</span>
          <div className="count">
            <span>{webhooks?.length ?? 0}</span>
          </div>
        </div>
        <div className="table-controls">
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
          <Button
            className="add-webhook"
            onClick={() => setWebhookAddModalState({ visible: true })}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text="Add new"
            icon={<SvgIconPlusWhite />}
          />
        </div>
      </motion.section>
      {breakpoint !== 'mobile' ? (
        filteredWebhooks && filteredWebhooks.length ? (
          <WebhooksTable columns={tableColumns} data={filteredWebhooks} />
        ) : (
          <NoData customMessage="No webhooks registered" />
        )
      ) : null}
      {filteredWebhooks && breakpoint === 'mobile' ? (
        <WebhooksList webhooks={filteredWebhooks} />
      ) : null}
      <AddWebhookModal />
    </PageContainer>
  );
};

export default WebhooksPage;

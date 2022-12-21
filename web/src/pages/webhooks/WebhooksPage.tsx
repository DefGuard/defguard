import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import { clone, orderBy } from 'lodash-es';
import { useEffect, useState } from 'react';
import useBreakpoint from 'use-breakpoint';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/components/layout/Button/Button';
import NoData from '../../shared/components/layout/NoData/NoData';
import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { Search } from '../../shared/components/layout/Search/Search';
import SvgIconPlusWhite from '../../shared/components/svg/IconPlusWhite';
import { deviceBreakpoints } from '../../shared/constants';
import { useModalStore } from '../../shared/hooks/store/useModalStore';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { Webhook } from '../../shared/types';
import { standardVariants } from '../../shared/variants';
import AddWebhookModal from './modals/AddWebhookModal/AddWebhookModal';
import WebhooksList from './WebhooksList/WebhooksList';

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
            initialValue={searchValue}
            onChange={(value) => setSearchValue(value)}
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
              placeholder="Find webhooks"
              onChange={(value) => setSearchValue(value)}
              initialValue={searchValue}
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
      {filteredWebhooks && filteredWebhooks.length > 0 && (
        <WebhooksList webhooks={filteredWebhooks} />
      )}
      {!filteredWebhooks ||
        (filteredWebhooks.length === 0 && (
          <NoData customMessage="No webhooks registered" />
        ))}
      <AddWebhookModal />
    </PageContainer>
  );
};

export default WebhooksPage;

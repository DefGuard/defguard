import './style.scss';

import { motion } from 'framer-motion';
import React from 'react';

import NoData from '../../../../shared/components/layout/NoData/NoData';
import { OpenidClient } from '../../../../shared/types';
import { tableRowVariants } from '../../../../shared/variants';
import OpenidClientListItem from './OpenidClientListItem';

interface Props {
  clients: OpenidClient[];
}

const OpenidClientListMobile: React.FC<Props> = ({ clients }) => {
  if (clients.length === 0) return <NoData customMessage="No apps found" />;

  return (
    <ul className="clients-list-mobile">
      {clients.map((client, index) => (
        <motion.li
          key={client.name}
          custom={index}
          variants={tableRowVariants}
          initial="hidden"
          animate="idle"
        >
          <OpenidClientListItem client={client} />
        </motion.li>
      ))}
    </ul>
  );
};

export default OpenidClientListMobile;

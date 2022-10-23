import './style.scss';

import { HTMLMotionProps, motion } from 'framer-motion';
import React from 'react';

import { OpenidClient } from '../../../../shared/types';
import OpenidClientInfo from './OpenidClientInfo';

interface Props {
  client: OpenidClient;
}

const OpenidClientDetail: React.FC<HTMLMotionProps<'div'> & Props> = ({
  client,
  ...rest
}) => {
  return (
    <motion.div className="client-details container-with-title" {...rest}>
      <div className="container">
        <OpenidClientInfo client={client} />
      </div>
    </motion.div>
  );
};

export default OpenidClientDetail;

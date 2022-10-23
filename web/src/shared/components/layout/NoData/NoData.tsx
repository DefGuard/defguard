import './style.scss';

import React from 'react';

interface Props {
  customMessage?: string;
}

/**
 * Styled placeholder for places where elements are waiting or has no data coming form API
 * @param customMessage Text to replace default 'No data' text
 */
const NoData: React.FC<Props> = ({ customMessage }) => {
  return (
    <p className="no-data">
      {customMessage && customMessage.length ? customMessage : 'No data'}
    </p>
  );
};

export default NoData;

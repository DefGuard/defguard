import { motion } from 'framer-motion';
import { useState } from 'react';

import SvgIconKey from '../../../../../../../shared/components/svg/IconKey';
import { ColorsRGB } from '../../../../../../../shared/constants';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { RowBox } from '../../../../../../../shared/defguard-ui/components/Layout/RowBox/RowBox';
import { SecurityKey } from '../../../../../../../shared/types';

interface KeyRowProps {
  data: SecurityKey;
  onDelete: () => void;
  disableDelete: boolean;
}
export const WebAuthNKeyRow = ({ data, onDelete, disableDelete }: KeyRowProps) => {
  return (
    <RowBox className="security-key">
      <SvgIconKey />
      <p>{data.name}</p>
      <Button
        styleVariant={ButtonStyleVariant.ICON}
        icon={<DeleteKeyIcon />}
        onClick={onDelete}
        disabled={disableDelete}
      />
    </RowBox>
  );
};

const DeleteKeyIcon = () => {
  const [hovered, setHovered] = useState(false);
  return (
    <motion.svg
      xmlns="http://www.w3.org/2000/svg"
      width={22}
      height={22}
      role="img"
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
    >
      <g transform="translate(-581 -1461)">
        <motion.g
          data-name="Group 4725"
          initial="idle"
          animate={hovered ? 'hover' : 'idle'}
          variants={{
            idle: {
              fill: '#899ca8',
            },
            hover: {
              fill: ColorsRGB.Error,
            },
          }}
        >
          <path
            data-name="Path 5669"
            d="M597.996 1467.459a1 1 0 0 0-1.07.924l-.835 11.376h-7.9l-.835-11.376a1 1 0 1 0-1.994.147l.9 12.3a1 1 0 0 0 1 .927h9.755a1 1 0 0 0 1-.927l.9-12.3a1 1 0 0 0-.921-1.071Z"
          />
          <path
            data-name="Path 5670"
            d="M599.285 1465.138h-3.546l-.846-2.463a1 1 0 0 0-.945-.675h-4.01a1 1 0 0 0-.945.675l-.846 2.463H585a1 1 0 0 0 0 2h14.285a1 1 0 0 0 0-2Zm-8.633-1.138h2.582l.391 1.138h-3.364Z"
          />
        </motion.g>
      </g>
    </motion.svg>
  );
};

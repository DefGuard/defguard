import React, { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import useBreakpoint from 'use-breakpoint';

import SvgIconInfoNormal from '../../../../../shared/components/svg/IconInfoNormal';
import SvgImageRegularNetwork from '../../../../../shared/components/svg/ImageRegularNetwork';
import { deviceBreakpoints } from '../../../../../shared/constants';
import NetworkSelectButton from './NetworkSelectButton';

interface Props extends React.HTMLAttributes<HTMLInputElement> {
  onChange: (value: unknown) => void;
  value: 'mesh' | 'regular';
}

// eslint-disable-next-line react/display-name
const RegularNetwork = React.forwardRef<HTMLInputElement, Props>(
  ({ onChange, value, ...props }, ref) => {
    const { t } = useTranslation('en');
    const { breakpoint } = useBreakpoint(deviceBreakpoints);
    const getClassName = useMemo(() => {
      const res = ['regular-network'];
      if (value === 'regular') {
        res.push('active');
      }
      if (value === 'mesh') {
        res.push('not-active');
      }
      return res.join(' ');
    }, [value]);

    const handleChange = () => {
      onChange('regular');
    };

    return (
      <div className={getClassName}>
        {breakpoint !== 'desktop' && <SvgIconInfoNormal />}
        <h3>{t('wizard.networkType.regularNetwork.title')}</h3>
        {breakpoint === 'desktop' && (
          <>
            <p>{t('wizard.networkType.regularNetwork.description')}</p>
            <SvgImageRegularNetwork />
          </>
        )}

        <NetworkSelectButton
          active={typeof value !== 'undefined' && value === 'regular'}
          onClick={() => handleChange()}
        />
        <input
          type="radio"
          ref={ref}
          {...props}
          onChange={onChange}
          checked={value === 'regular'}
        />
      </div>
    );
  }
);

export default RegularNetwork;

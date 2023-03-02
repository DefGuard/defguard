import React, { useMemo } from 'react';
import { useI18nContext } from '../../../../../i18n/i18n-react';
import useBreakpoint from 'use-breakpoint';

import SvgIconInfoNormal from '../../../../../shared/components/svg/IconInfoNormal';
import SvgImageRegularNetwork from '../../../../../shared/components/svg/ImageRegularNetwork';
import { deviceBreakpoints } from '../../../../../shared/constants';
import NetworkSelectButton from './NetworkSelectButton';

interface Props extends React.HTMLAttributes<HTMLInputElement> {
  onChange: (value: unknown) => void;
  value: 'import' | 'regular';
}

// eslint-disable-next-line react/display-name
const RegularNetwork = React.forwardRef<HTMLInputElement, Props>(
  ({ onChange, value, ...props }, ref) => {
    const { LL } = useI18nContext();
    const { breakpoint } = useBreakpoint(deviceBreakpoints);
    const getClassName = useMemo(() => {
      const res = ['regular-network'];
      if (value === 'regular') {
        res.push('active');
      }
      if (value === 'import') {
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
        <h3>{LL.wizard.networkType.regularNetwork.title()}</h3>
        {breakpoint === 'desktop' && (
          <>
            <p>{LL.wizard.networkType.regularNetwork.description()}</p>
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

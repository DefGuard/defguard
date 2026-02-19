import { type ReactNode, useMemo } from 'react';
import './style.scss';
import clsx from 'clsx';
import { range, sum } from 'lodash-es';
import React from 'react';
import { SectionMarker } from '../../defguard-ui/components/SectionMarker/SectionMarker';
import { ThemeVariable } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';

interface Props {
  name?: string;
  children: ReactNode;
}

export const IpAssignmentDeviceSection = ({ name, children }: Props) => {
  const childrenCount = useMemo(() => {
    return React.Children.count(children);
  }, [children]);

  // returns error heights in order of children to calc accurate heights of lines positioners later
  const errorInfo = useMemo(() => {
    return React.Children.toArray(children).map((child) =>
      // @ts-expect-error
      React.isValidElement(child) && isPresent(child.props?.error) ? 24 : 0,
    );
  }, [children]);

  // calc main line height
  const mainLineHeight = useMemo(() => {
    const gapHeight = 8;
    const baseHeight = 38;
    const halfHeight = baseHeight / 2;
    let gaps = 0;
    let res = 0;
    if (childrenCount > 0) {
      gaps = gapHeight * (childrenCount - 1);
      res += gaps;
      if (childrenCount === 1) {
        return res + halfHeight;
      }
      res += halfHeight;
      res += baseHeight * (childrenCount - 1);
    }
    // add errors
    res += sum(errorInfo);
    return res;
  }, [childrenCount, errorInfo]);

  return (
    <div className="ip-assignment-device-section">
      {isPresent(name) && (
        <div className="top-track">
          <SectionMarker icon="devices" />
          <p className="device-name">{name}</p>
        </div>
      )}
      <div className="inputs-track">
        <div className="lines">
          {childrenCount > 0 &&
            range(childrenCount).map((index) => {
              const errorHeight = errorInfo[index];
              const isLast = index === childrenCount - 1;
              return (
                <div
                  key={index}
                  className={clsx('line', {
                    'is-last': isLast,
                  })}
                  style={{
                    height: 38 + errorHeight,
                  }}
                >
                  <ArrowIcon />
                </div>
              );
            })}
          <div
            className="main-line"
            style={{
              height: mainLineHeight - 4,
            }}
          ></div>
        </div>
        <div className="inputs">{children}</div>
      </div>
    </div>
  );
};

const ArrowIcon = () => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="17"
      height="12"
      viewBox="0 0 17 12"
      fill="none"
    >
      <path
        d="M16.8536 7.85355C17.0488 7.65829 17.0488 7.34171 16.8536 7.14645L13.6716 3.96447C13.4763 3.7692 13.1597 3.7692 12.9645 3.96447C12.7692 4.15973 12.7692 4.47631 12.9645 4.67157L15.7929 7.5L12.9645 10.3284C12.7692 10.5237 12.7692 10.8403 12.9645 11.0355C13.1597 11.2308 13.4763 11.2308 13.6716 11.0355L16.8536 7.85355ZM0.5 0H0V1.5H0.5H1V0H0.5ZM6.5 7.5V8H16.5V7.5V7H6.5V7.5ZM0.5 1.5H0C0 5.08985 2.91015 8 6.5 8V7.5V7C3.46243 7 1 4.53757 1 1.5H0.5Z"
        fill="#DFE3E9"
        style={{
          fill: ThemeVariable.BorderMuted,
        }}
      />
    </svg>
  );
};

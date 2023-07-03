import './style.scss';

import { ComponentPropsWithoutRef, useMemo } from 'react';

import { ActivityIcon } from '../ActivityIcon/ActivityIcon';

interface Props extends ComponentPropsWithoutRef<'div'> {
  connectionStatus?: ActivityType;
  customMessage?: string;
}

export enum ActivityType {
  CONNECTED = 'connected',
  ADDED = 'added',
  REMOVED = 'removed',
  DISCONNECTED = 'disconnected',
  ALERT = 'alert',
}

/**
 * Displays styled information about connection status of an device or user.
 */
export const ActivityStatus = ({
  connectionStatus = ActivityType.CONNECTED,
  customMessage,
  className,
  ...rest
}: Props) => {
  const getText = useMemo(() => {
    switch (connectionStatus) {
      case ActivityType.CONNECTED:
        return 'Connected';
      case ActivityType.ADDED:
        return 'New device';
      case ActivityType.ALERT:
        return 'Heavy usage alert';
      case ActivityType.DISCONNECTED:
        return 'Disconnected';
      case ActivityType.REMOVED:
        return 'Removed device';
    }
  }, [connectionStatus]);

  const getClassName = useMemo(() => {
    const res = ['activity-status'];
    res.push(connectionStatus.valueOf());
    if (className) {
      res.push(className);
    }
    return res.join(' ');
  }, [className, connectionStatus]);

  return (
    <div className={getClassName} {...rest}>
      <ActivityIcon status={connectionStatus} />
      <span>{customMessage || getText}</span>
    </div>
  );
};

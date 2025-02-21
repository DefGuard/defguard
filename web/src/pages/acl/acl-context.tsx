/* eslint-disable react-refresh/only-export-components */
import { useCallback, useState } from 'react';
import { createContainer } from 'react-tracked';

import { AclCreateContext, AclCreateContextLoaded } from './types';

const init: AclCreateContext = {
  devices: undefined,
  groups: undefined,
  networks: undefined,
  users: undefined,
};

const useValue = () => useState(init);

const {
  useUpdate,
  Provider: AclCreateTrackedProvider,
  useTrackedState,
  useSelector: useAclCreateSelector,
} = createContainer(useValue);

const useUpdateAclCreateContext = () => {
  const updateInner = useUpdate();

  const update = useCallback(
    (values: Partial<AclCreateContext>) => {
      updateInner((s) => ({ ...s, ...values }));
    },
    [updateInner],
  );

  return update;
};

const useAclLoadedContext = () => {
  const { devices, groups, networks, users } = useTrackedState();
  if (
    devices === undefined ||
    groups === undefined ||
    networks === undefined ||
    users === undefined
  ) {
    throw Error('Use of ACL data before it was loaded');
  }
  return {
    devices,
    groups,
    networks,
    users,
  } as AclCreateContextLoaded;
};

export {
  AclCreateTrackedProvider,
  useAclCreateSelector,
  useAclLoadedContext,
  useUpdateAclCreateContext,
};

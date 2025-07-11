/* eslint-disable react-refresh/only-export-components */
import { useCallback, useState } from 'react';
import { createContainer } from 'react-tracked';

import type { AclCreateContext, AclCreateContextLoaded } from './types';

const init: AclCreateContext = {
  devices: undefined,
  groups: undefined,
  networks: undefined,
  users: undefined,
  editRule: undefined,
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
  const { devices, groups, networks, users, editRule, aliases } = useTrackedState();

  if (
    devices === undefined ||
    groups === undefined ||
    networks === undefined ||
    users === undefined ||
    aliases === undefined
  ) {
    throw Error('Use of ACL data before it was loaded');
  }
  return {
    devices,
    groups,
    networks,
    users,
    aliases,
    ruleToEdit: editRule,
  } as AclCreateContextLoaded;
};

export {
  AclCreateTrackedProvider,
  useAclCreateSelector,
  useAclLoadedContext,
  useUpdateAclCreateContext,
};

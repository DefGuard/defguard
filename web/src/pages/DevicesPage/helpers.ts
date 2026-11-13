import { m } from '../../paraglide/messages';
import type { PolicyOsVariant } from '../../shared/components/SystemSelector/types';
import type { IconKindValue } from '../../shared/defguard-ui/components/Icon';
import { policyOsVariantToText } from '../../shared/utils/policyPostures';
import { mobileOsTypes } from './consts';

export const isMobileOs = (osType: PolicyOsVariant) => mobileOsTypes.includes(osType);

export const platformIcon = (osType: PolicyOsVariant): IconKindValue =>
  isMobileOs(osType) ? 'mobile' : 'desktop';

export const platformText = (osType: PolicyOsVariant) =>
  isMobileOs(osType) ? m.devices_platform_mobile() : m.devices_platform_desktop();

export const systemText = (osType: PolicyOsVariant, osVersion: string | null) =>
  [policyOsVariantToText(osType), osVersion].filter(Boolean).join(' ');

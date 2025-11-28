import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState } from 'react';
import { type SubmitHandler, useForm, useWatch } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormAclDefaultPolicy } from '../../../../shared/components/Form/FormAclDefaultPolicySelect/FormAclDefaultPolicy.tsx';
import { FormLocationMfaModeSelect } from '../../../../shared/components/Form/FormLocationMfaModeSelect/FormLocationMfaModeSelect.tsx';
import { FormServiceLocationModeSelect } from '../../../../shared/components/Form/FormServiceLocationModeSelect/FormServiceLocationModeSelect.tsx';
import { RenderMarkdown } from '../../../../shared/components/Layout/RenderMarkdown/RenderMarkdown.tsx';
import { FormCheckBox } from '../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox.tsx';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types.ts';
import type { SelectOption } from '../../../../shared/defguard-ui/components/Layout/Select/types';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore.ts';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../shared/queries';
import { LocationMfaMode, ServiceLocationMode } from '../../../../shared/types.ts';
import { titleCase } from '../../../../shared/utils/titleCase';
import { trimObjectStrings } from '../../../../shared/utils/trimObjectStrings.ts';
import { Validate } from '../../../../shared/validators';
import { useWizardStore } from '../../hooks/useWizardStore';
import { DividerHeader } from './components/DividerHeader.tsx';

export const WizardNetworkConfiguration = () => {
  const [componentMount, setComponentMount] = useState(false);
  const [groupOptions, setGroupOptions] = useState<SelectOption<string>[]>([]);
  const submitRef = useRef<HTMLInputElement | null>(null);
  const {
    network: { addNetwork },
    groups: { getGroups },
  } = useApi();

  const [submitSubject, nextSubject, setWizardState] = useWizardStore(
    (state) => [state.submitSubject, state.nextStepSubject, state.setState],
    shallow,
  );

  const wizardNetworkConfiguration = useWizardStore((state) => state.manualNetworkConfig);
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);

  const toaster = useToaster();
  const { LL } = useI18nContext();

  const { mutate: addNetworkMutation, isPending } = useMutation({
    mutationFn: addNetwork,
    onSuccess: () => {
      setWizardState({ loading: false });
      toaster.success(LL.wizard.configuration.successMessage());
      nextSubject.next();
    },
    onError: (err) => {
      setWizardState({ loading: false });
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const {
    isError: isFetchGroupsError,
    error: fetchGroupsError,
    isLoading: groupsLoading,
    data: fetchGroupsData,
  } = useQuery({
    queryKey: [QueryKeys.FETCH_GROUPS],
    queryFn: getGroups,
    enabled: componentMount,
    refetchOnMount: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: 'always',
  });

  // biome-ignore lint/correctness/useExhaustiveDependencies: migration, checkMeLater
  useEffect(() => {
    if (fetchGroupsError) {
      toaster.error(LL.messages.error());
      console.error(fetchGroupsError);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fetchGroupsError]);

  useEffect(() => {
    if (fetchGroupsData) {
      setGroupOptions(
        fetchGroupsData.groups.map((g) => ({
          key: g,
          value: g,
          label: titleCase(g),
        })),
      );
    }
  }, [fetchGroupsData]);

  const zodSchema = useMemo(
    () =>
      z.object({
        name: z.string().trim().min(1, LL.form.error.required()),
        address: z
          .string()
          .trim()
          .min(1, LL.form.error.required())
          .refine((val) => {
            for (const address of val.split(',')) {
              if (!(Validate.CIDRv4(address) || Validate.CIDRv6(address))) {
                return false;
              }
            }
            return true;
          }, LL.form.error.addressNetmask()),
        endpoint: z
          .string()
          .trim()
          .min(1, LL.form.error.required())
          .refine((val) => {
            if (val.split(',').length > 1) {
              return false; // for now we can only accept one gateway address
            }
            return Validate.IPv4(val) || Validate.IPv6(val) || Validate.Domain(val);
          }, LL.form.error.endpoint()),
        port: z
          .number({
            invalid_type_error: LL.form.error.invalid(),
          })
          .max(65535, LL.form.error.portMax())
          .nonnegative(),
        allowed_ips: z
          .string()
          .trim()
          .refine((val) => {
            for (const address of val.split(',')) {
              if (
                !(
                  Validate.CIDRv4(address) ||
                  Validate.IPv4(address) ||
                  Validate.CIDRv6(address) ||
                  Validate.IPv6(address)
                )
              ) {
                return false;
              }
            }
            return true;
          }, LL.form.error.address()),
        dns: z
          .string()
          .trim()
          .optional()
          .refine((val) => {
            if (val === '' || !val) {
              return true;
            }
            return Validate.IPv4(val) || Validate.IPv6(val);
          }, LL.form.error.address()),
        allowed_groups: z.array(z.string().trim().min(1, LL.form.error.minimumLength())),
        keepalive_interval: z
          .number({
            invalid_type_error: LL.form.error.invalid(),
          })
          .positive(),
        peer_disconnect_threshold: z
          .number({
            invalid_type_error: LL.form.error.invalid(),
          })
          .refine((v) => v >= 120, LL.form.error.minimumLength()),
        acl_enabled: z.boolean(),
        acl_default_allow: z.boolean(),
        location_mfa_mode: z.nativeEnum(LocationMfaMode),
        service_location_mode: z.nativeEnum(ServiceLocationMode),
      }),
    [LL.form.error],
  );

  type FormInputs = z.infer<typeof zodSchema>;

  const getDefaultValues = useMemo((): FormInputs => {
    return { ...wizardNetworkConfiguration, allowed_groups: [] };
  }, [wizardNetworkConfiguration]);

  const { handleSubmit, control } = useForm<FormInputs>({
    mode: 'all',
    defaultValues: getDefaultValues,
    resolver: zodResolver(zodSchema),
  });

  const aclEnabled = useWatch({
    control,
    name: 'acl_enabled',
    defaultValue: getDefaultValues.acl_enabled,
  });
  const locationMfaMode = useWatch({
    control,
    name: 'location_mfa_mode',
    defaultValue: getDefaultValues.location_mfa_mode,
  });
  const mfaDisabled = useMemo(
    () => locationMfaMode === LocationMfaMode.DISABLED,
    [locationMfaMode],
  );
  const serviceLocationMode = useWatch({
    control,
    name: 'service_location_mode',
    defaultValue: getDefaultValues.service_location_mode,
  });
  const serviceLocationEnabled = useMemo(
    () => serviceLocationMode !== ServiceLocationMode.DISABLED,
    [serviceLocationMode],
  );

  const handleValidSubmit: SubmitHandler<FormInputs> = (values) => {
    const trimmed = trimObjectStrings(values);
    if (!isPending) {
      setWizardState({ loading: true });
      addNetworkMutation(trimmed);
    }
  };

  useEffect(() => {
    const sub = submitSubject.subscribe(() => {
      submitRef.current?.click();
    });
    return () => sub?.unsubscribe();
  }, [submitSubject]);

  useEffect(() => {
    setTimeout(() => setComponentMount(true), 100);
  }, []);

  return (
    <Card id="wizard-manual-network-configuration" shaded>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          controller={{ control, name: 'name' }}
          label={LL.networkConfiguration.form.fields.name.label()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.address()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'address' }}
          label={LL.networkConfiguration.form.fields.address.label()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.gateway()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'endpoint' }}
          label={LL.networkConfiguration.form.fields.endpoint.label()}
        />
        <FormInput
          controller={{ control, name: 'port' }}
          label={LL.networkConfiguration.form.fields.port.label()}
          type="number"
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.allowedIps()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'allowed_ips' }}
          label={LL.networkConfiguration.form.fields.allowedIps.label()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.dns()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'dns' }}
          label={LL.networkConfiguration.form.fields.dns.label()}
        />
        <FormInput
          controller={{ control, name: 'keepalive_interval' }}
          label={LL.networkConfiguration.form.fields.keepalive_interval.label()}
          type="number"
        />
        <DividerHeader
          text={LL.networkConfiguration.form.sections.accessControl.header()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.allowedGroups()}</p>
        </MessageBox>
        <FormSelect
          controller={{ control, name: 'allowed_groups' }}
          label={LL.networkConfiguration.form.fields.allowedGroups.label()}
          loading={groupsLoading}
          disabled={isFetchGroupsError || (!groupsLoading && groupOptions.length === 0)}
          options={groupOptions}
          placeholder={LL.networkConfiguration.form.fields.allowedGroups.placeholder()}
          renderSelected={(group) => ({
            key: group,
            displayValue: titleCase(group),
          })}
        />
        {!enterpriseEnabled && (
          <MessageBox type={MessageBoxType.WARNING}>
            <p>{LL.networkConfiguration.form.helpers.aclFeatureDisabled()}</p>
          </MessageBox>
        )}
        <FormCheckBox
          controller={{ control, name: 'acl_enabled' }}
          label={LL.networkConfiguration.form.fields.acl_enabled.label()}
          labelPlacement="right"
        />
        <FormAclDefaultPolicy
          controller={{ control, name: 'acl_default_allow' }}
          disabled={!aclEnabled}
        />
        <DividerHeader text={LL.networkConfiguration.form.sections.mfa.header()} />
        <MessageBox id="location-mfa-mode-explain-message-box">
          <p>{LL.networkConfiguration.form.helpers.locationMfaMode.description()}</p>
          <ul>
            <li>
              <p>{LL.networkConfiguration.form.helpers.locationMfaMode.internal()}</p>
            </li>
            <li>
              <RenderMarkdown
                content={LL.networkConfiguration.form.helpers.locationMfaMode.external()}
              />
            </li>
          </ul>
        </MessageBox>
        {serviceLocationEnabled && (
          <MessageBox type={MessageBoxType.WARNING}>
            <p>
              {LL.networkConfiguration.form.helpers.locationMfaMode.serviceLocationWarning()}
            </p>
          </MessageBox>
        )}
        <FormLocationMfaModeSelect
          controller={{ control, name: 'location_mfa_mode' }}
          disabled={serviceLocationEnabled}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.peerDisconnectThreshold()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'peer_disconnect_threshold' }}
          label={LL.networkConfiguration.form.fields.peer_disconnect_threshold.label()}
          type="number"
          disabled={mfaDisabled}
        />
        {!mfaDisabled && (
          <MessageBox type={MessageBoxType.WARNING}>
            <p>{LL.networkConfiguration.form.helpers.serviceLocation.mfaWarning()}</p>
          </MessageBox>
        )}
        <FormServiceLocationModeSelect
          controller={{ control, name: 'service_location_mode' }}
          disabled={!mfaDisabled}
        />
        <input type="submit" className="visually-hidden" ref={submitRef} />
      </form>
    </Card>
  );
};

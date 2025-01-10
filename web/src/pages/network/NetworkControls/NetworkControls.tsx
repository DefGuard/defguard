import './style.scss';

import { useMutation } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { useBreakpoint } from 'use-breakpoint';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../shared/components/svg/IconCheckmarkWhite';
import SvgIconX from '../../../shared/components/svg/IconX';
import { deviceBreakpoints } from '../../../shared/constants';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/defguard-ui/components/Layout/Button/types';
import { ConfirmModal } from '../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import { Select } from '../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
} from '../../../shared/defguard-ui/components/Layout/Select/types';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { useWizardStore } from '../../wizard/hooks/useWizardStore';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

export const NetworkControls = () => {
  const {
    network: { deleteNetwork },
  } = useApi();
  const toaster = useToaster();
  const [isDeleteModalOpen, setDeleteModalOpen] = useState(false);
  const navigate = useNavigate();
  const resetWizardState = useWizardStore((state) => state.resetState);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const { LL } = useI18nContext();

  const [save, setNetworkState] = useNetworkPageStore(
    (state) => [state.saveSubject, state.setState],
    shallow,
  );

  const [loading, selectedNetworkId] = useNetworkPageStore(
    (state) => [state.loading, state.selectedNetworkId],
    shallow,
  );

  const networks = useNetworkPageStore((state) => state.networks);

  const getOptions = useMemo(
    (): SelectOption<number>[] =>
      networks.map((n) => ({
        value: n.id,
        label: n.name,
        key: n.id,
      })),
    [networks],
  );

  const selectedNetwork = networks.find((n) => n.id === selectedNetworkId);

  const { isPending: isLoading, mutate: deleteNetworkMutate } = useMutation({
    mutationFn: deleteNetwork,
    onSuccess: () => {
      toaster.success(LL.networkConfiguration.messages.delete.success());
      navigate('/admin/overview', { replace: true });
    },
    onError: (err) => {
      toaster.error(LL.networkConfiguration.messages.delete.error());
      console.error(err);
    },
  });

  const renderSelected = useCallback(
    (networkId: number): SelectSelectedValue => {
      const selectedOption = getOptions.find((o) => o.value === networkId);

      if (selectedOption) {
        return {
          key: networkId,
          displayValue: selectedOption.label,
        };
      }

      return {
        key: 'none',
        displayValue: 'Error',
      };
    },
    [getOptions],
  );

  return (
    <>
      <div className="network-controls">
        {breakpoint !== 'desktop' && (
          <div className="network-select">
            <Select
              renderSelected={renderSelected}
              selected={selectedNetworkId}
              options={getOptions}
              addOptionLabel={LL.networkPage.addNetwork()}
              label={LL.networkPage.controls.networkSelect.label()}
              onChangeSingle={(res) => setNetworkState({ selectedNetworkId: res })}
              onCreate={() => {
                resetWizardState();
                navigate('/admin/wizard', { replace: true });
              }}
            />
          </div>
        )}
        <Button
          className="cancel"
          text={LL.networkConfiguration.form.controls.cancel()}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.LINK}
          onClick={() => navigate('/admin/overview', { replace: true })}
        />
        <Button
          data-testid="delete-network"
          text={LL.networkConfiguration.form.controls.delete()}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.CONFIRM}
          onClick={() => setDeleteModalOpen(true)}
          icon={<SvgIconX />}
        />
        <Button
          className="submit"
          text={LL.networkConfiguration.form.controls.submit()}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          icon={<IconCheckmarkWhite />}
          loading={loading}
          onClick={() => save.next()}
        />
      </div>
      <ConfirmModal
        type={ConfirmModalType.WARNING}
        isOpen={isDeleteModalOpen}
        setIsOpen={(v) => setDeleteModalOpen(v)}
        onSubmit={() => deleteNetworkMutate(selectedNetworkId)}
        onCancel={() => setDeleteModalOpen(false)}
        title={LL.modals.deleteNetwork.title({
          name: selectedNetwork?.name || '',
        })}
        subTitle={LL.modals.deleteNetwork.subTitle()}
        submitText={LL.modals.deleteNetwork.submit()}
        cancelText={LL.modals.deleteNetwork.cancel()}
        loading={isLoading}
      />
    </>
  );
};

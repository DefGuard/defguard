import './style.scss';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useEffect, useState } from 'react';
import { Subject } from 'rxjs';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import SvgIconCheckmark from '../../../../../../../shared/components/svg/IconCheckmark';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Label } from '../../../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { LoaderSpinner } from '../../../../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { NoData } from '../../../../../../../shared/defguard-ui/components/Layout/NoData/NoData';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../../shared/queries';
import { Provisioner, WorkerJobStatus } from '../../../../../../../shared/types';
import { useAddAuthorizationKeyModal } from '../../useAddAuthorizationKeyModal';
import { ProvisionerRow } from './components/ProvisionerRow';

export const AddAuthenticationKeyYubikey = () => {
  const { LL } = useI18nContext();
  const {
    provisioning: { getWorkers, provisionYubiKey, getJobStatus },
  } = useApi();

  const { data, isLoading: workersListLoading } = useQuery({
    queryKey: [QueryKeys.FETCH_WORKERS],
    queryFn: getWorkers,
    refetchInterval: 1000,
  });

  const queryClient = useQueryClient();
  const setModalState = useAddAuthorizationKeyModal((s) => s.setState, shallow);
  const localLL = LL.userPage.authenticationKeys.addModal.yubikeyForm;
  const isProvisioning = useAddAuthorizationKeyModal((s) => s.provisioningInProgress);
  const user = useAddAuthorizationKeyModal((s) => s.user);
  const toaster = useToaster();
  const closeModal = useAddAuthorizationKeyModal((s) => s.close, shallow);

  const [selectedWorker, setSelectedWorker] = useState<Provisioner | undefined>(
    undefined,
  );

  const [workerJobId, setWorkerJob] = useState<number | undefined>(undefined);

  const [statusSubject] = useState(new Subject<WorkerJobStatus | null>());

  const { data: workerJobStatus } = useQuery({
    queryFn: () => getJobStatus(workerJobId as number),
    queryKey: [QueryKeys.FETCH_WORKER_JOB_STATUS, workerJobId],
    enabled: !isUndefined(workerJobId) && isProvisioning,
    refetchInterval: 1000,
  });

  const { mutate: createJob } = useMutation({
    mutationFn: provisionYubiKey,
    onSuccess: (res) => {
      setWorkerJob(res.id);
    },
    onError: (e) => {
      setModalState({ provisioningInProgress: false });
      console.error(e);
    },
  });

  const handleProvision = () => {
    if (!isProvisioning && user && selectedWorker) {
      setModalState({
        provisioningInProgress: true,
      });
      createJob({ username: user.username, worker: selectedWorker.id });
    }
  };

  // send results to Subject
  useEffect(() => {
    if (workerJobStatus !== undefined) {
      statusSubject.next(workerJobStatus);
    }
  }, [statusSubject, workerJobStatus]);

  // handle last known status
  useEffect(() => {
    const sub = statusSubject.subscribe((jobStatus) => {
      if (jobStatus) {
        if (jobStatus.success) {
          toaster.success(localLL.provisioning.success());
          const invalidate = [
            QueryKeys.FETCH_USERS_LIST,
            QueryKeys.FETCH_AUTHENTICATION_KEYS_INFO,
            QueryKeys.FETCH_USER_PROFILE,
          ];
          invalidate.forEach((k) =>
            queryClient.invalidateQueries({
              queryKey: [k],
            }),
          );
          closeModal();
        }
        if (jobStatus.errorMessage) {
          toaster.error(localLL.provisioning.error());
          toaster.error(jobStatus.errorMessage);
          closeModal();
        }
      }
    });

    return () => {
      sub.unsubscribe();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [statusSubject]);

  return (
    <div className="add-yubikey-wrapper">
      {!isProvisioning && (
        <div className="provisioners-list">
          {data && data.length > 0 && <Label>{localLL.selectWorker.selectLabel()}</Label>}
          {data &&
            data.length > 0 &&
            data.map((w) => (
              <ProvisionerRow
                name={w.id}
                available={w.connected}
                key={w.id}
                selected={w.id === selectedWorker?.id}
                onClick={() => {
                  if (w.connected) {
                    if (w.id === selectedWorker?.id) {
                      setSelectedWorker(undefined);
                    } else {
                      setSelectedWorker(w);
                    }
                  }
                }}
              />
            ))}
          {workersListLoading && (
            <div className="loader-wrapper">
              <LoaderSpinner size={120} />
            </div>
          )}
          {!workersListLoading && data?.length === 0 && (
            <NoData customMessage={localLL.selectWorker.noData()} />
          )}
        </div>
      )}
      {isProvisioning && (
        <div className="loader-wrapper">
          <LoaderSpinner size={120} />
          <NoData customMessage={localLL.provisioning.inProgress()} />
        </div>
      )}
      <div className="controls">
        <Button
          type="button"
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.common.controls.cancel()}
          onClick={() => closeModal()}
          disabled={isProvisioning}
        />
        <Button
          type="submit"
          icon={<SvgIconCheckmark />}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          loading={isProvisioning}
          text={localLL.submit()}
          disabled={isUndefined(selectedWorker)}
          onClick={() => handleProvision()}
        />
      </div>
    </div>
  );
};

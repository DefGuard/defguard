import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { motion } from 'framer-motion';
import { useEffect, useState } from 'react';
import { Subject, switchMap, timer } from 'rxjs';
import { useBreakpoint } from 'use-breakpoint';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconButton from '../../../../../shared/components/layout/IconButton/IconButton';
import { LoaderSpinner } from '../../../../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import MessageBox, {
  MessageBoxType,
} from '../../../../../shared/components/layout/MessageBox/MessageBox';
import { Modal } from '../../../../../shared/components/layout/Modal/Modal';
import { IconHamburgerClose } from '../../../../../shared/components/svg';
import SvgIconCancel from '../../../../../shared/components/svg/IconCancel';
import { deviceBreakpoints } from '../../../../../shared/constants';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { WorkerJobStatus, WorkerJobStatusError } from '../../../../../shared/types';
import WorkerLoader from './WorkerLoader/WorkerLoader';
import { WorkerSelectionForm } from './WorkerSelectionForm/WorkerSelectionForm';

export const KeyProvisioningModal = () => {
  const { LL } = useI18nContext();
  const [{ visible: isOpen, user: selectedUser }, setModalState] = useModalStore(
    (state) => [state.provisionKeyModal, state.setProvisionKeyModal],
    shallow,
  );
  const setIsOpen = (v: boolean) => {
    setModalState({ visible: v });
  };
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const [noWorkers, setNoWorkers] = useState(false);
  const [jobStatusSubject, setJobStatusSubject] = useState<Subject<void> | undefined>();
  const [submitted, setSubmitted] = useState(false);
  const [jobSucceeded, setJobSucceeded] = useState(false);
  const [errorData, setErrorData] = useState('');
  const [keysData, setKeysData] = useState<
    Pick<WorkerJobStatus, 'pgp_cert_id' | 'pgp_key' | 'ssh_key'> | undefined
  >();
  const [jobId, setJobId] = useState<number | undefined>();
  const {
    provisioning: { getWorkers, getJobStatus },
  } = useApi();
  const queryClient = useQueryClient();
  const toaster = useToaster();

  const { data: workers, isLoading } = useQuery([QueryKeys.FETCH_WORKERS], getWorkers, {
    onSuccess: (data) => {
      if (!data || (data && !data.length)) {
        setNoWorkers(true);
        setTimeout(() => queryClient.invalidateQueries([QueryKeys.FETCH_WORKERS]), 2500);
      } else {
        if (noWorkers) {
          setNoWorkers(false);
        }
      }
    },
    onError: (err) => {
      setIsOpen(false);
      toaster.error(LL.messages.error());
      console.error(err);
    },
    enabled: isOpen,
  });
  useQuery([QueryKeys.FETCH_WORKER_JOB_STATUS, jobId], () => getJobStatus(jobId), {
    enabled: typeof jobId === 'number' && isOpen,
    refetchOnWindowFocus: false,
    onSuccess: (data) => {
      if (typeof data !== 'undefined' && data) {
        if (data.success === true) {
          const { success, ...rest } = data;
          if (success === true) {
            setJobSucceeded(true);
            setKeysData(rest);
            toaster.success(LL.modals.provisionKeys.messages.success());
          } else {
            toaster.error(LL.messages.error());
          }
        }
      } else {
        jobStatusSubject?.next();
      }
    },
    onError: (err: AxiosError<WorkerJobStatusError>) => {
      const data: WorkerJobStatusError | undefined = err.response?.data;
      if (data && data.message && data.message.length) {
        setJobSucceeded(true);
        setErrorData(data.message);
      } else {
        setErrorData(LL.modals.provisionKeys.messages.errorStatus());
      }
    },
  });

  useEffect(() => {
    if (!isOpen) {
      setSubmitted(false);
      setJobId(undefined);
      setKeysData(undefined);
      setJobSucceeded(false);
      setErrorData('');
    } else {
      queryClient.invalidateQueries([QueryKeys.FETCH_WORKERS]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, queryClient]);

  useEffect(() => {
    if (!jobStatusSubject) {
      setJobStatusSubject(new Subject());
    } else {
      const sub = jobStatusSubject.pipe(switchMap(() => timer(2500))).subscribe(() => {
        queryClient.invalidateQueries([QueryKeys.FETCH_WORKER_JOB_STATUS]);
      });
      return () => sub.unsubscribe();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [jobStatusSubject]);

  const afterSubmit = (jobId: number) => {
    setJobId(jobId);
    setSubmitted(true);
  };

  return (
    <Modal
      backdrop
      setIsOpen={setIsOpen}
      isOpen={isOpen}
      className={
        submitted ? 'key-provisioning middle submitted' : 'key-provisioning middle'
      }
    >
      {!submitted ? (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          style={{
            height: '100%',
            display: 'flex',
            flexDirection: 'column',
          }}
        >
          <section className="provisioning-top">
            <header>
              <p>
                {LL.modals.provisionKeys.title()}
                <span className="user"> {selectedUser?.username}</span>
              </p>
              <IconButton
                className="blank"
                whileHover={{ scale: 1.2 }}
                onClick={() => setIsOpen(false)}
              >
                {breakpoint !== 'desktop' ? <IconHamburgerClose /> : null}
                {breakpoint === 'desktop' ? <SvgIconCancel /> : null}
              </IconButton>
            </header>
            <MessageBox type={MessageBoxType.INFO}>
              <p
                dangerouslySetInnerHTML={{
                  __html: LL.modals.provisionKeys.infoBox(),
                }}
              ></p>
            </MessageBox>
          </section>
          {isLoading || !workers || (workers && !workers.length) ? (
            <div className="loader">
              <LoaderSpinner size={80} />
              <p>{LL.modals.provisionKeys.noData.workers()}</p>
            </div>
          ) : (
            <WorkerSelectionForm
              setIsOpen={setIsOpen}
              afterSubmit={afterSubmit}
              workers={workers}
            />
          )}
        </motion.div>
      ) : null}
      {submitted ? (
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          style={{ width: '100%', height: '100%', position: 'relative' }}
        >
          <WorkerLoader
            setIsOpen={setIsOpen}
            succeeded={jobSucceeded}
            errorData={errorData}
            keyData={keysData}
          />
        </motion.div>
      ) : null}
    </Modal>
  );
};

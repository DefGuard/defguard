import { m } from '../../../paraglide/messages';
import { Divider } from '../../defguard-ui/components/Divider/Divider';
import { Modal } from '../../defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../defguard-ui/components/ModalControls/ModalControls';
import { Search } from '../../defguard-ui/components/Search/Search';
import { ThemeSpacing } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../hooks/modalControls/modalTypes';
import type { OpenDisplayListModal } from '../../hooks/modalControls/types';
import './style.scss';
import { useEffect, useMemo, useState } from 'react';

const modalNameValue = ModalName.DisplayList;

type ModalData = OpenDisplayListModal;

export const DisplayListModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      id="display-list-modal"
      title={'Details'}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && <ModalContent {...modalData} />}
    </Modal>
  );
};

const itemHeight = 24;
const itemGap = 8;

const ModalContent = ({ data }: ModalData) => {
  const [search, setSearch] = useState('');
  const maxHeight = useMemo(() => itemHeight * 10 + itemGap * 9, []);

  const distilledData = useMemo(() => {
    let res = data;
    if (search.length) {
      res = res.filter((item) => item.toLowerCase().includes(search.toLowerCase()));
    }
    return res;
  }, [data, search]);

  return (
    <>
      {data.length >= 10 && (
        <>
          <Search
            initialValue={search}
            onChange={setSearch}
            placeholder={m.controls_search()}
          />
          <Divider spacing={ThemeSpacing.Xl} />
        </>
      )}
      <ul
        style={{
          maxHeight,
        }}
      >
        {distilledData.map((text, index) => (
          <li key={index}>{text}</li>
        ))}
      </ul>
      <ModalControls
        submitProps={{
          text: m.controls_close(),
          onClick: () => {
            closeModal(modalNameValue);
          },
        }}
      />
    </>
  );
};

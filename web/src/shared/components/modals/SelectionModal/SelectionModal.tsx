import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Modal } from '../../../defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../defguard-ui/components/ModalControls/ModalControls';
import { SelectionSection } from '../../SelectionSection/SelectionSection';
import type { SelectionSectionKey } from '../../SelectionSection/type';
import { useSelectionModal } from './useSelectionModal';

export const SelectionModal = () => {
  const title = useSelectionModal((s) => s.title);
  const isOpen = useSelectionModal((s) => s.isOpen);

  useEffect(() => {
    return () => {
      useSelectionModal.getState().reset();
    };
  }, []);

  return (
    <Modal
      title={title}
      id="selection-modal"
      isOpen={isOpen}
      onClose={() => {
        useSelectionModal.setState({ isOpen: false });
      }}
      afterClose={() => {
        useSelectionModal.getState().reset();
      }}
    >
      <ModalContent />
    </Modal>
  );
};

const ModalContent = () => {
  const options = useSelectionModal((s) => s.options);
  const initialSelected = useSelectionModal((s) => s.selected);

  const [internalSelection, setInternalSelection] =
    useState<Set<SelectionSectionKey>>(initialSelected);

  return (
    <>
      <SelectionSection
        options={options}
        selection={internalSelection}
        onChange={setInternalSelection}
      />
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          onClick: () => {
            useSelectionModal.setState({ isOpen: false });
          },
        }}
        submitProps={{
          text: m.controls_submit(),
          onClick: () => {
            useSelectionModal.getState().onSubmit?.(Array.from(internalSelection));
            useSelectionModal.setState({
              isOpen: false,
            });
          },
        }}
      />
    </>
  );
};

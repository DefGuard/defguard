import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Modal } from '../../../defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../defguard-ui/components/ModalControls/ModalControls';
import { SelectionSection } from '../../SelectionSection/SelectionSection';
import type { SelectionKey } from '../../SelectionSection/type';
import { useSelectionModal } from './useSelectionModal';

export const SelectionModal = () => {
  const title = useSelectionModal((s) => s.title);
  const contentClassName = useSelectionModal((s) => s.contentClassName);
  const isOpen = useSelectionModal((s) => s.isOpen);
  const onCancel = useSelectionModal((s) => s.onCancel);

  useEffect(() => {
    return () => {
      useSelectionModal.getState().reset();
    };
  }, []);

  return (
    <Modal
      title={title}
      id="selection-modal"
      contentClassName={contentClassName}
      isOpen={isOpen}
      onClose={() => {
        useSelectionModal.setState({ isOpen: false });
        onCancel?.();
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
  const renderItem = useSelectionModal((s) => s.renderItem);
  const orderItems = useSelectionModal((s) => s.orderItems);
  const searchPlaceholder = useSelectionModal((s) => s.searchPlaceholder);
  const visibleItemsLimit = useSelectionModal((s) => s.visibleItemsLimit);
  const itemGap = useSelectionModal((s) => s.itemGap);
  const enableDividers = useSelectionModal((s) => s.enableDividers);

  const [internalSelection, setInternalSelection] =
    useState<Set<SelectionKey>>(initialSelected);

  return (
    <>
      <SelectionSection
        options={options}
        selection={internalSelection}
        onChange={setInternalSelection}
        renderItem={renderItem}
        orderItems={orderItems}
        searchPlaceholder={searchPlaceholder}
        visibleItemsLimit={visibleItemsLimit}
        itemGap={itemGap}
        enableDividers={enableDividers}
      />
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          onClick: () => {
            useSelectionModal.setState({ isOpen: false });
            useSelectionModal.getState().onCancel?.();
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

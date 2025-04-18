import './style.scss';

import useResizeObserver from '@react-hook/resize-observer';
import clsx from 'clsx';
import { useCallback, useMemo, useRef, useState } from 'react';

import { FieldError } from '../../../../../shared/defguard-ui/components/Layout/FieldError/FieldError';
import { FloatingMenu } from '../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenu';
import { FloatingMenuProvider } from '../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuProvider';
import { FloatingMenuTrigger } from '../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuTrigger';
import { Label } from '../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import { DialogSelectButtonIcon } from './DialogSelectButtonIcon';
import { DialogSelectModal } from './DialogSelectModal/DialogSelectModal';
import { DialogSelectProps } from './types';

export const DialogSelect = <T extends object, I extends number | string>({
  options,
  selected,
  identKey,
  label,
  onChange,
  renderTagContent,
  renderDialogListItem,
  searchFn,
  searchKeys,
  errorMessage,
  modalExtrasTop,
  disabled = false,
}: DialogSelectProps<T, I>) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [overflows, setOverflows] = useState(false);

  const handleResize = useCallback(() => {
    if (containerRef.current) {
      setOverflows(containerRef.current.scrollWidth > containerRef.current.clientWidth);
    }
  }, []);

  useResizeObserver(containerRef, handleResize);
  const [modalOpen, setModalOpen] = useState(false);
  const getIdent = useCallback((val: T): I => val[identKey] as I, [identKey]);

  const selectedOptions = useMemo(
    () => options.filter((o) => selected.includes(getIdent(o))),
    [getIdent, options, selected],
  );

  const error = !disabled ? errorMessage : undefined;

  const getLabel = renderDialogListItem ? renderDialogListItem : renderTagContent;

  return (
    <>
      <div className="dialog-select spacer">
        <div className="inner">
          {label !== undefined && <Label>{label}</Label>}
          <div
            className={clsx('dialog-select-field', {
              disabled,
              invalid: isPresent(error),
            })}
          >
            <FloatingMenuProvider placement="top">
              <FloatingMenuTrigger asChild>
                <div
                  className={clsx('track', {
                    overflows,
                  })}
                  ref={containerRef}
                >
                  <div className="options">
                    {renderTagContent !== undefined &&
                      selectedOptions.map((o) => {
                        const id = getIdent(o);
                        return (
                          <div className="dialog-select-tag" key={id}>
                            {renderTagContent(o)}
                          </div>
                        );
                      })}
                  </div>
                </div>
              </FloatingMenuTrigger>
              <FloatingMenu className="dialog-select-track-floating-menu">
                <ul>
                  {selectedOptions.map((o) => {
                    const id = getIdent(o);
                    return <li key={id}>{getLabel(o)}</li>;
                  })}
                </ul>
              </FloatingMenu>
            </FloatingMenuProvider>
            <button
              disabled={disabled}
              className="open-button"
              onClick={() => {
                setModalOpen(true);
              }}
              type="button"
            >
              <DialogSelectButtonIcon />
            </button>
          </div>
          <FieldError errorMessage={error} />
        </div>
      </div>
      <DialogSelectModal
        searchFn={searchFn}
        searchKeys={searchKeys}
        open={modalOpen}
        setOpen={setModalOpen}
        options={options}
        getIdent={getIdent}
        initiallySelected={selected}
        getLabel={getLabel}
        extrasTop={modalExtrasTop}
        onChange={(vals) => {
          onChange?.(vals);
        }}
      />
    </>
  );
};

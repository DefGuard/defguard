import {
  autoUpdate,
  FloatingPortal,
  offset,
  shift,
  size,
  useClick,
  useDismiss,
  useFloating,
  useInteractions,
} from '@floating-ui/react';
import { useState } from 'react';
import type { ButtonProps } from '../../../defguard-ui/components/Button/types';
import { ButtonMenu } from '../../../defguard-ui/components/ButtonMenu/MenuButton';
import { Menu } from '../../../defguard-ui/components/Menu/Menu';
import type { MenuItemsGroup } from '../../../defguard-ui/components/Menu/types';

/** Properties for the standard button menu variant. */
type ButtonVariantProps = Omit<ButtonProps, 'ref'> & {
  kind: 'button';
  options: MenuItemsGroup[];
};

/** Properties for the compact text menu variant. */
type PlainVariantProps = {
  kind: 'plain';
  options: MenuItemsGroup[];
  label: string;
};

type MfaMethodsMenuProps = ButtonVariantProps | PlainVariantProps;

export const MfaMethodsMenu = (props: MfaMethodsMenuProps) => {
  if (props.kind === 'button') {
    const { kind, options, ...buttonProps } = props;
    return <ButtonMenu menuItems={options} {...buttonProps} />;
  }

  return <PlainButton options={props.options} label={props.label} />;
};

/** Renders the compact add-method menu trigger used inside a step. */
const PlainButton = ({
  options,
  label,
}: {
  options: MenuItemsGroup[];
  label: string;
}) => {
  const [isOpen, setOpen] = useState(false);
  const { refs, context, floatingStyles } = useFloating({
    placement: 'bottom-start',
    whileElementsMounted: autoUpdate,
    onOpenChange: setOpen,
    open: isOpen,
    middleware: [
      offset(4),
      shift(),
      size({
        apply({ rects, elements, availableHeight }) {
          elements.floating.style.minWidth = `${rects.reference.width}px`;
          elements.floating.style.maxHeight = `${availableHeight - 10}px`;
        },
      }),
    ],
  });

  const click = useClick(context, { toggle: true });
  const dismiss = useDismiss(context, {
    ancestorScroll: true,
    escapeKey: true,
    outsidePress: (event) => !(event.target as HTMLElement).closest('.menu'),
  });
  const { getFloatingProps, getReferenceProps } = useInteractions([click, dismiss]);

  return (
    <>
      <button
        type="button"
        className="plain-button"
        ref={refs.setReference}
        {...getReferenceProps()}
      >
        <span>{label}</span>
      </button>
      {isOpen && (
        <FloatingPortal>
          <Menu
            itemGroups={options}
            ref={refs.setFloating}
            style={floatingStyles}
            onClose={() => setOpen(false)}
            {...getFloatingProps()}
          />
        </FloatingPortal>
      )}
    </>
  );
};

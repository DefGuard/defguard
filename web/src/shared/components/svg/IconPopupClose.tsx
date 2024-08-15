import type { SVGProps } from 'react';
const SvgIconPopupClose = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-popup-close_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            fill: '#fff',
            opacity: 0,
          }}
        />
      </clipPath>
      <style>{'.icon-popup-close_svg__c{fill:#fff}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-popup-close_svg__a)',
      }}
      transform="rotate(90 11 11)"
    >
      <rect
        width={16}
        height={2}
        className="icon-popup-close_svg__c"
        rx={1}
        transform="rotate(45 -2.571 9.621)"
      />
      <rect
        width={16}
        height={2}
        className="icon-popup-close_svg__c"
        rx={1}
        transform="rotate(135 7.429 6.621)"
      />
    </g>
  </svg>
);
export default SvgIconPopupClose;

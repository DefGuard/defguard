import type { SVGProps } from 'react';
const SvgIconArrowGrayDown = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-arrow-gray-down_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            fill: '#899ca8',
            opacity: 0,
          }}
        />
      </clipPath>
      <style>{'.icon-arrow-gray-down_svg__c{fill:#899ca8}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-arrow-gray-down_svg__a)',
      }}
      transform="rotate(-90 11 11)"
    >
      <rect
        width={8}
        height={2}
        className="icon-arrow-gray-down_svg__c"
        rx={1}
        transform="rotate(45 -7.4 14.862)"
      />
      <rect
        width={8}
        height={2}
        className="icon-arrow-gray-down_svg__c"
        rx={1}
        transform="rotate(135 5.672 6.106)"
      />
    </g>
  </svg>
);
export default SvgIconArrowGrayDown;

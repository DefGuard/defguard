import type { SVGProps } from 'react';
const SvgIconArrowGrayUp1 = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-arrow-gray-up-1_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#899ca8',
          }}
        />
      </clipPath>
      <style>{'.icon-arrow-gray-up-1_svg__c{fill:#899ca8}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-arrow-gray-up-1_svg__a)',
      }}
      transform="rotate(-90 11 11)"
    >
      <rect
        width={8}
        height={2}
        className="icon-arrow-gray-up-1_svg__c"
        rx={1}
        transform="rotate(-45 16.742 -2.863)"
      />
      <rect
        width={8}
        height={2}
        className="icon-arrow-gray-up-1_svg__c"
        rx={1}
        transform="rotate(-135 9.814 5.893)"
      />
    </g>
  </svg>
);
export default SvgIconArrowGrayUp1;

import type { SVGProps } from 'react';
const SvgIconPlusWhite = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-plus-white_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            fill: '#617684',
            opacity: 0,
          }}
        />
      </clipPath>
      <style>{'.icon-plus-white_svg__c{fill:#617684}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-plus-white_svg__a)',
      }}
    >
      <rect
        width={10}
        height={2}
        className="icon-plus-white_svg__c"
        rx={1}
        transform="rotate(-90 13 3)"
      />
      <rect
        width={10}
        height={2}
        className="icon-plus-white_svg__c"
        rx={1}
        transform="rotate(-180 8 6)"
      />
    </g>
  </svg>
);
export default SvgIconPlusWhite;

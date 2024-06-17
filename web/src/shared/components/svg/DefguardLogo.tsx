import type { SVGProps } from 'react';
const SvgDefguardLogo = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={13}
    height={27}
    fill="none"
    viewBox="0 0 13 27"
    {...props}
  >
    <g clipPath="url(#defguard-logo_svg__a)">
      <path
        fill="#0C8CE0"
        d="M11.58 0v5.849L6.517 2.924 0 6.687v14.206l6.512 3.762 5.063-2.924v3.014l-2.45 1.417 1.449.838 2.45-1.417V13.368L6.512 9.606 1.449 12.53V7.52l5.063-2.925 5.063 2.924v1.665l1.449.838V.838L11.574 0zM1.45 20.055v-5.011l5.063 2.924 5.063-2.924v5.011L6.512 22.98zm9.404-6.265-4.341 2.508L2.17 13.79l4.34-2.508z"
      />
    </g>
    <defs>
      <clipPath id="defguard-logo_svg__a">
        <path fill="#fff" d="M0 0h13v27H0z" />
      </clipPath>
    </defs>
  </svg>
);
export default SvgDefguardLogo;

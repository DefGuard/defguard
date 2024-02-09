import type { SVGProps } from 'react';
const SvgIconHourglass = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-hourglass_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            fill: '#899ca8',
            opacity: 0,
          }}
        />
      </clipPath>
      <style>{'.icon-hourglass_svg__c{fill:#899ca8}'}</style>
    </defs>
    <path
      d="M59.094 14.75h-.625v-2.311A5.07 5.07 0 0 0 55.849 8a5.07 5.07 0 0 0 2.62-4.439V1.25h.625a.625.625 0 0 0 0-1.25H47.25a.625.625 0 0 0 0 1.25h.594v2.311A5.07 5.07 0 0 0 50.465 8a5.07 5.07 0 0 0-2.62 4.439v2.311h-.595a.625.625 0 0 0 0 1.25h11.844a.625.625 0 0 0 0-1.25m-10-11.189V1.25h8.125v2.311a3.81 3.81 0 0 1-3.8 3.814 3.86 3.86 0 0 1-4.325-3.814m0 8.878a3.86 3.86 0 0 1 4.328-3.814 3.81 3.81 0 0 1 3.8 3.814v2.311h-8.128Zm6.188-7.533h-4.25a.625.625 0 0 1 0-1.25h4.25a.625.625 0 1 1 0 1.25m.319 8.44a.625.625 0 0 0 0-.884l-1.315-1.306a1.51 1.51 0 0 0-2.125 0l-1.316 1.306a.625.625 0 1 0 .881.887l1.316-1.307a.26.26 0 0 1 .363 0l1.316 1.306a.625.625 0 0 0 .884 0Z"
      className="icon-hourglass_svg__c"
      style={{
        clipPath: 'url(#icon-hourglass_svg__a)',
      }}
      transform="translate(-41.99 3)"
    />
  </svg>
);
export default SvgIconHourglass;

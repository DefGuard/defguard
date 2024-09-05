/* eslint-disable max-len */
import type { BaseTranslation } from '../i18n-types';

const ko: BaseTranslation = {
  common: {
    conditions: {
      or: '또는',
      and: '그리고',
      equal: '같음',
    },
    controls: {
      next: '다음',
      back: '뒤로',
      cancel: '취소',
      confirm: '확인',
      submit: '제출',
      close: '닫기',
      select: '선택',
      finish: '완료',
      saveChanges: '변경 사항 저장',
      save: '저장',
      RestoreDefault: '기본값 복원',
      delete: '삭제',
      rename: '이름 변경',
      copy: '복사',
      edit: '편집',
    },
    key: '키',
    name: '이름',
  },
  messages: {
    error: '오류가 발생했습니다.',
    success: '작업이 성공했습니다',
    errorVersion: '애플리케이션 버전을 가져오지 못했습니다.',
    insecureContext: '컨텍스트가 안전하지 않습니다.',
    details: '상세내용:',
    clipboard: {
      error: '클립보드에 액세스할 수 없습니다.',
      success: '클립보드에 복사되었습니다.',
    },
  },
  modals: {
    addGroup: {
      title: '그룹 추가',
      selectAll: '모든 사용자 선택',
      groupName: '그룹 이름',
      searchPlaceholder: '필터/검색',
      submit: '그룹 생성',
    },
    editGroup: {
      title: '그룹 편집',
      selectAll: '모든 사용자 선택',
      groupName: '그룹 이름',
      searchPlaceholder: '필터/검색',
      submit: '그룹 업데이트',
    },
    deleteGroup: {
      title: '{name:string} 그룹 삭제',
      subTitle: '이 작업은 이 그룹을 영구적으로 삭제합니다.',
      locationListHeader: '이 그룹은 현재 다음 VPN 위치에 할당되어 있습니다:',
      locationListFooter: `이것이 주어진 위치에 허용된 유일한 그룹인 경우, 해당 위치는 <b>모든 사용자가 액세스할 수 있게</b> 됩니다.`,
      submit: '그룹 삭제',
      cancel: '취소',
    },
    deviceConfig: {
      title: '장치 VPN 구성',
    },
    changePasswordSelf: {
      title: '비밀번호 변경',
      messages: {
        success: '비밀번호가 변경되었습니다',
        error: '비밀번호 변경에 실패했습니다',
      },
      form: {
        labels: {
          newPassword: '새 비밀번호',
          oldPassword: '현재 비밀번호',
          repeat: '새 비밀번호 확인',
        },
      },
      controls: {
        submit: '비밀번호 변경',
        cancel: '취소',
      },
    },
    startEnrollment: {
      title: '등록 시작',
      desktopTitle: '데스크톱 활성화',
      messages: {
        success: '사용자 등록이 시작되었습니다',
        successDesktop: '데스크톱 구성이 시작되었습니다',
        error: '사용자 등록을 시작하지 못했습니다',
        errorDesktop: '데스크톱 활성화를 시작하지 못했습니다',
      },
      form: {
        email: {
          label: '이메일',
        },
        mode: {
          options: {
            email: '이메일로 토큰 보내기',
            manual: '직접 토큰 전달',
          },
        },
        submit: '등록 시작',
        submitDesktop: '데스크톱 활성화',
        smtpDisabled: '이메일로 토큰을 보내려면 SMTP를 구성하십시오. 설정 -> SMTP로 이동하십시오.',
      },
      tokenCard: {
        title: '활성화 토큰',
      },
      urlCard: {
        title: 'Defguard 인스턴스 URL',
      },
    },
    deleteNetwork: {
      title: '{name:string} 위치 삭제',
      subTitle: '이 작업은 이 위치를 영구적으로 삭제합니다.',
      submit: '위치 삭제',
      cancel: '취소',
    },
    changeWebhook: {
      messages: {
        success: 'Webhook이 변경되었습니다.',
      },
    },
    manageWebAuthNKeys: {
      title: '보안 키',
      messages: {
        deleted: 'WebAuthN 키가 삭제되었습니다.',
        duplicateKeyError: '키가 이미 등록되어 있습니다',
      },
      infoMessage: `
        <p>
          보안 키는 인증 코드 대신 2단계 인증으로 사용될 수 있습니다.

          보안 키 구성에 대해 자세히 알아보세요.
        </p>
`,
      form: {
        messages: {
          success: '보안 키가 추가되었습니다.',
        },
        fields: {
          name: {
            label: '새 키 이름',
          },
        },
        controls: {
          submit: '새 키 추가',
        },
      },
    },
    recoveryCodes: {
      title: '복구 코드',
      submit: '코드를 저장했습니다',
      messages: {
        copied: '코드가 복사되었습니다.',
      },
      infoMessage: `
        <p>
          복구 코드는 비밀번호와 동일한 수준의 주의를 기울여 취급하십시오!

          Lastpass, bitwarden 또는 Keeper와 같은 비밀번호 관리자를 사용하여 저장하는 것을 권장합니다.
        </p>
`,
    },
    registerTOTP: {
      title: 'Authenticator 앱 설정',
      infoMessage: `
        <p>
          MFA를 설정하려면, 이 QR 코드를 인증 앱으로 스캔한 다음,
          아래 필드에 코드를 입력하세요:
        </p>
`,
      messages: {
        totpCopied: 'TOTP 경로가 복사되었습니다.',
        success: 'TOTP가 활성화되었습니다',
      },
      copyPath: 'TOTP 경로 복사',
      form: {
        fields: {
          code: {
            label: 'Authenticator 코드',
            error: '코드가 유효하지 않습니다',
          },
        },
        controls: {
          submit: '코드 확인',
        },
      },
    },
    registerEmailMFA: {
      title: '이메일 MFA 설정',
      infoMessage: `
        <p>
          MFA를 설정하려면 계정 이메일: <strong>{email: string}</strong>로 전송된 코드를 입력하세요
        </p>
`,
      messages: {
        success: '이메일 MFA가 활성화되었습니다',
        resend: '인증 코드가 재전송되었습니다',
      },
      form: {
        fields: {
          code: {
            label: '이메일 코드',
            error: '코드가 유효하지 않습니다',
          },
        },
        controls: {
          submit: '코드 확인',
          resend: '이메일 재전송',
        },
      },
    },
    editDevice: {
      title: '장치 편집',
      messages: {
        success: '장치가 업데이트되었습니다.',
      },
      form: {
        fields: {
          name: {
            label: '장치 이름',
          },
          publicKey: {
            label: '장치 공개 키 (WireGuard)',
          },
        },
        controls: {
          submit: '장치 편집',
        },
      },
    },
    deleteDevice: {
      title: '장치 삭제',
      message: '{deviceName} 장치를 삭제하시겠습니까?',
      submit: '장치 삭제',
      messages: {
        success: '장치가 삭제되었습니다.',
      },
    },
    addWallet: {
      title: '지갑 추가',
      infoBox: 'ETH 지갑을 추가하려면 메시지에 서명해야 합니다.',
      form: {
        fields: {
          name: {
            placeholder: '지갑 이름',
            label: '이름',
          },
          address: {
            placeholder: '지갑 주소',
            label: '주소',
          },
        },
        controls: {
          submit: '지갑 추가',
        },
      },
    },
    keyDetails: {
      title: 'YubiKey 세부 정보',
      downloadAll: '모든 키 다운로드',
    },
    deleteUser: {
      title: '계정 삭제',
      controls: {
        submit: '계정 삭제',
      },
      message: '{username: string} 계정을 영구적으로 삭제하시겠습니까?',
      messages: {
        success: '{username: string}이(가) 삭제되었습니다.',
      },
    },
    disableUser: {
      title: '계정 비활성화',
      controls: {
        submit: '계정 비활성화',
      },
      message: '{username: string} 계정을 비활성화하시겠습니까?',
      messages: {
        success: '{username: string}이(가) 비활성화되었습니다.',
      },
    },
    enableUser: {
      title: '계정 활성화',
      controls: {
        submit: '계정 활성화',
      },
      message: '{username: string} 계정을 활성화하시겠습니까?',
      messages: {
        success: '{username: string}이(가) 활성화되었습니다.',
      },
    },
    deleteProvisioner: {
      title: '프로비저너 삭제',
      controls: {
        submit: '프로비저너 삭제',
      },
      message: '{id: string} 프로비저너를 삭제하시겠습니까?',
      messages: {
        success: '{provisioner: string}이(가) 삭제되었습니다.',
      },
    },
    changeUserPassword: {
      messages: {
        success: '비밀번호가 변경되었습니다.',
      },
      title: '사용자 비밀번호 변경',
      form: {
        controls: {
          submit: '새 비밀번호 저장',
        },
        fields: {
          newPassword: {
            label: '새 비밀번호',
          },
          confirmPassword: {
            label: '비밀번호 다시 입력',
          },
        },
      },
    },
    provisionKeys: {
      title: 'Yubikey 프로비저닝:',
      warning:
        '이 작업은 yubikey의 openpgp 애플리케이션을 삭제하고 재구성합니다.',
      infoBox: `선택한 프로비저너에는 프로비저닝할 <b>깨끗한</b> YubiKey가
                연결되어 있어야 합니다. 사용된 YubiKey를 청소하려면 프로비저닝하기 전에
                <b>gpg --card-edit </b>를 실행하십시오.`,
      selectionLabel: '다음 프로비저너 중 하나를 선택하여 YubiKey를 프로비저닝하십시오:',
      noData: {
        workers: '작업자를 찾을 수 없습니다. 대기 중...',
      },
      controls: {
        submit: 'YubiKey 프로비저닝',
      },
      messages: {
        success: '키가 프로비저닝되었습니다',
        errorStatus: '작업자 상태를 가져오는 중 오류가 발생했습니다.',
      },
    },
    addUser: {
      title: '새 사용자 추가',
      messages: {
        userAdded: '사용자가 추가되었습니다',
      },
      form: {
        submit: '사용자 추가',
        fields: {
          username: {
            placeholder: '로그인',
            label: '로그인',
          },
          password: {
            placeholder: '비밀번호',
            label: '비밀번호',
          },
          email: {
            placeholder: '사용자 이메일',
            label: '사용자 이메일',
          },
          firstName: {
            placeholder: '이름',
            label: '이름',
          },
          lastName: {
            placeholder: '성',
            label: '성',
          },
          phone: {
            placeholder: '전화번호',
            label: '전화번호',
          },
          enableEnrollment: {
            label: '등록 프로세스 사용',
            link: '<a href="https://defguard.gitbook.io/defguard/help/enrollment" target="_blank">자세한 정보는 여기를 참고하세요</a>',
          },
        },
      },
    },
    webhookModal: {
      title: {
        addWebhook: '웹훅 추가.',
        editWebhook: '웹훅 편집',
      },
      messages: {
        clientIdCopy: '클라이언트 ID가 복사되었습니다.',
        clientSecretCopy: '클라이언트 암호가 복사되었습니다.',
      },
      form: {
        triggers: '트리거 이벤트:',
        messages: {
          successAdd: '웹훅이 생성되었습니다.',
          successModify: '웹훅이 수정되었습니다.',
        },
        error: {
          urlRequired: 'URL이 필요합니다.',
          validUrl: '유효한 URL이어야 합니다.',
          scopeValidation: '최소 하나의 트리거가 있어야 합니다.',
          tokenRequired: '토큰이 필요합니다.',
        },
        fields: {
          description: {
            label: '설명',
            placeholder: '새 사용자 생성 시 gmail 계정을 생성하는 웹훅',
          },
          token: {
            label: '비밀 토큰',
            placeholder: '인증 토큰',
          },
          url: {
            label: '웹훅 URL',
            placeholder: 'https://example.com/webhook',
          },
          userCreated: {
            label: '새 사용자 생성됨',
          },
          userDeleted: {
            label: '사용자 삭제됨',
          },
          userModified: {
            label: '사용자 수정됨',
          },
          hwkeyProvision: {
            label: '사용자 Yubikey 프로비저닝',
          },
        },
      },
    },
    deleteWebhook: {
      title: '웹훅 삭제',
      message: '{name: string} 웹훅을 삭제하시겠습니까?',
      submit: '삭제',
      messages: {
        success: '웹훅이 삭제되었습니다.',
      },
    },
  },
  addDevicePage: {
    title: '장치 추가',
    helpers: {
      setupOpt: `이 마법사를 사용하여 장치를 추가할 수 있습니다. 당사의 기본 애플리케이션인 "defguard" 또는 다른 WireGuard 클라이언트를 선택하세요. 잘 모르시겠다면 간편하게 defguard를 사용하는 것을 권장합니다.`,
      client: `defguard 데스크톱 클라이언트는 <a href="https://defguard.net/download" target="_blank">여기</a>에서 다운로드하고 <a href="https://defguard.gitbook.io/defguard/help/configuring-vpn/add-new-instance" target="_blank">이 가이드</a>를 따르세요.`,
    },
    messages: {
      deviceAdded: '장치가 추가되었습니다',
    },
    steps: {
      setupMethod: {
        remote: {
          title: '데스크톱 클라이언트 구성',
          subTitle:
            '단일 토큰으로 간편하게 설정할 수 있습니다. 클라이언트를 다운로드하고 간단한 보안을 즐기세요.',
          link: 'defguard 클라이언트 다운로드',
        },
        manual: {
          title: '수동 WireGuard 클라이언트',
          subTitle:
            '고급 사용자의 경우 다운로드 또는 QR 코드를 통해 고유한 구성을 얻으세요. 클라이언트를 다운로드하고 VPN 설정을 제어하세요.',
          link: 'WireGuard 클라이언트 다운로드',
        },
      },
      configDevice: {
        title: '장치 구성',
        messages: {
          copyConfig: '구성이 클립보드에 복사되었습니다',
    },
        helpers: {
          warningAutoMode: `
    <p>
      개인 키를 저장하지 않으므로
      지금 구성을 다운로드해야 합니다.
      이 페이지가 닫히면 전체 구성 파일(개인 키 포함, 빈 템플릿만)을
      가져올 수 없습니다.
    </p>
`,
          warningManualMode: `
    <p>
      여기에 제공된 구성에는 개인 키가 포함되어 있지 않으며 공개 키를 사용하여 채워져 있습니다. 구성이 제대로 작동하려면 직접 교체해야 합니다.
    </p>
`,
          warningNoNetworks: "액세스할 수 있는 네트워크가 없습니다.",
          qrHelper: `
      <p>
        이 QR 코드를 스캔하여 wireguard 애플리케이션으로 장치를 더 빠르게 설정할 수 있습니다.
      </p>`,
        },
        qrInfo:
          '아래 제공된 구성 파일을 QR 코드를 스캔하거나 장치의 WireGuard 인스턴스에 파일로 가져와서 사용하세요.',
        inputNameLabel: '장치 이름',
        qrLabel: 'WireGuard 구성 파일',
      },
      setupDevice: {
        title: 'VPN 장치 생성',
        infoMessage: `
        <p>
          장치에서 WireGuardVPN을 구성해야 합니다. 방법을 모르는 경우&nbsp;
          <a href="{addDevicesDocs:string}">문서</a>를 참조하세요.
        </p>
`,
        options: {
          auto: '키 쌍 생성',
          manual: '내 공개 키 사용',
        },
        form: {
          fields: {
            name: {
              label: '장치 이름',
            },
            publicKey: {
              label: '공개 키 제공',
            },
          },
          errors: {
            name: {
              duplicatedName: '이 이름을 가진 장치가 이미 존재합니다',
            },
          },
        },
      },
      copyToken: {
        title: '클라이언트 활성화',
        tokenCardTitle: '활성화 토큰',
        urlCardTitle: 'Defguard 인스턴스 URL',
      },
    },
  },
  userPage: {
    title: {
      view: '사용자 프로필',
      edit: '사용자 프로필 편집',
    },
    messages: {
      editSuccess: '사용자가 업데이트되었습니다.',
      failedToFetchUserData: '사용자 정보를 가져올 수 없습니다.',
      passwordResetEmailSent: '비밀번호 재설정 이메일이 전송되었습니다.',
    },
    userDetails: {
      header: '프로필 세부 정보',
      messages: {
        deleteApp: '앱 및 모든 토큰이 삭제되었습니다.',
      },
      warningModals: {
        title: '경고',
        content: {
          usernameChange: `사용자 이름을 변경하면 Defguard를 사용하여 로그인한 서비스에 큰 영향을 미칩니다. 사용자 이름을 변경하면 사용자가 애플리케이션에 대한 액세스 권한을 잃을 수 있습니다(애플리케이션에서 해당 사용자를 인식하지 못하기 때문에). 계속 진행하시겠습니까?`,
          emailChange: `외부 OpenID Connect(OIDC) 공급자를 사용하여 사용자를 인증하는 경우 사용자의 이메일 주소를 변경하면 Defguard에 로그인하는 기능에 큰 영향을 미칠 수 있습니다. 계속 진행하시겠습니까?`,
        },
        buttons: {
          proceed: '진행',
          cancel: '취소',
        },
      },
      fields: {
        username: {
          label: '사용자 이름',
        },
        firstName: {
          label: '이름',
        },
        lastName: {
          label: '성',
        },
        phone: {
          label: '전화번호',
        },
        email: {
          label: '이메일',
        },
        status: {
          label: '상태',
          active: '활성',
          disabled: '비활성',
        },
        groups: {
          label: '사용자 그룹',
          noData: '그룹 없음',
        },
        apps: {
          label: '승인된 앱',
          noData: '승인된 앱 없음',
        },
      },
    },
    userAuthInfo: {
      header: '비밀번호 및 인증',
      password: {
        header: '비밀번호 설정',
        changePassword: '비밀번호 변경',
      },
      recovery: {
        header: '복구 옵션',
        codes: {
          label: '복구 코드',
          viewed: '조회됨',
        },
      },
      mfa: {
        header: '이중 인증 방법',
        edit: {
          disable: 'MFA 비활성화',
        },
        messages: {
          mfaDisabled: 'MFA가 비활성화되었습니다.',
          OTPDisabled: '일회용 비밀번호가 비활성화되었습니다.',
          EmailMFADisabled: '이메일 MFA가 비활성화되었습니다.',
          changeMFAMethod: 'MFA 방법이 변경되었습니다',
        },
        securityKey: {
          singular: '보안 키',
          plural: '보안 키',
        },
        default: '기본값',
        enabled: '활성화됨',
        disabled: '비활성화됨',
        wallet: {
          singular: '지갑',
          plural: '지갑',
        },
        labels: {
          totp: '시간 기반 일회용 비밀번호',
          email: '이메일',
          webauth: '보안 키',
          wallets: '지갑',
        },
        editMode: {
          enable: '활성화',
          disable: '비활성화',
          makeDefault: '기본값으로 설정',
          webauth: {
            manage: '보안 키 관리',
          },
        },
      },
    },
    controls: {
      editButton: '프로필 편집',
      deleteAccount: '계정 삭제',
    },
    devices: {
      header: '사용자 장치',
      addDevice: {
        web: '새 장치 추가',
        desktop: '이 장치 추가',
      },
      card: {
        labels: {
          publicIP: '공개 IP',
          connectedThrough: '연결 방식',
          connectionDate: '연결 날짜',
          lastLocation: '마지막 연결 위치',
          lastConnected: '마지막 연결',
          assignedIp: '할당된 IP',
          active: '활성',
          noData: '연결된 적 없음',
        },
        edit: {
          edit: '장치 편집',
          delete: '장치 삭제',
          showConfigurations: '구성 보기',
        },
      },
    },
    wallets: {
      messages: {
        addressCopied: '주소가 복사되었습니다.',
        duplicate: {
          primary: '연결된 지갑이 이미 등록되어 있습니다',
          sub: '사용되지 않은 지갑을 연결하세요.',
        },
      },
      header: '사용자 지갑',
      addWallet: '새 지갑 추가',
      card: {
        address: '주소',
        mfaBadge: 'MFA',
        edit: {
          enableMFA: 'MFA 활성화',
          disableMFA: 'MFA 비활성화',
          delete: '삭제',
          copyAddress: '주소 복사',
        },
        messages: {
          deleteSuccess: '지갑이 삭제되었습니다',
          enableMFA: '지갑 MFA가 활성화되었습니다',
          disableMFA: '지갑 MFA가 비활성화되었습니다',
        },
      },
    },
    yubiKey: {
      header: '사용자 YubiKey',
      provision: 'YubiKey 프로비저닝',
      keys: {
        pgp: 'PGP 키',
        ssh: 'SSH 키',
      },
      noLicense: {
        moduleName: 'YubiKey 모듈',
        line1: 'YubiKey 관리 및 프로비저닝을 위한 엔터프라이즈 모듈입니다.',
        line2: '',
      },
    },
    authenticationKeys: {
      header: '사용자 인증 키',
      addKey: '새 키 추가',
      keysList: {
        common: {
          rename: '이름 변경',
          key: '키',
          download: '다운로드',
          copy: '복사',
          serialNumber: '시리얼 번호',
          delete: '삭제',
        },
      },
      deleteModal: {
        title: '인증 키 삭제',
        confirmMessage: '{name: string} 키가 영구적으로 삭제됩니다.',
      },
      addModal: {
        header: '새 인증 키 추가',
        keyType: '키 유형',
        keyForm: {
          placeholders: {
            title: '키 이름',
            key: {
              ssh: 'ssh-rsa, ecdsa-sha2-nistp256, ... 로 시작',
              gpg: '-----BEGIN PGP PUBLIC KEY BLOCK----- 로 시작',
            },
          },
          labels: {
            title: '이름',
            key: '키',
          },
          submit: '{name: string} 키 추가',
        },
        yubikeyForm: {
          selectWorker: {
            info: '이 작업은 YubiKey의 openpgp 애플리케이션을 삭제하고 재구성합니다.',
            selectLabel: '다음 프로비저너 중 하나를 선택하여 YubiKey를 프로비저닝하십시오',
            noData: '현재 등록된 작업자가 없습니다.',
            available: '사용 가능',
            unavailable: '사용 불가',
          },
          provisioning: {
            inProgress: '프로비저닝 진행 중, 잠시 기다려 주세요.',
            error: '프로비저닝 실패!',
            success: 'Yubikey가 성공적으로 프로비저닝되었습니다',
          },
          submit: 'Yubikey 프로비저닝',
        },
        messages: {
          keyAdded: '키가 추가되었습니다.',
          keyExists: '키가 이미 추가되었습니다.',
          unsupportedKeyFormat: '지원되지 않는 키 형식입니다.',
          genericError: '키를 추가할 수 없습니다. 나중에 다시 시도하십시오.',
        },
      },
    },
  },
  usersOverview: {
    pageTitle: '사용자',
    search: {
      placeholder: '사용자 찾기',
    },
    filterLabels: {
      all: '모든 사용자',
      admin: '관리자만',
      users: '사용자만',
    },
    usersCount: '모든 사용자',
    addNewUser: '새로 추가',
    list: {
      headers: {
        name: '사용자 이름',
        username: '로그인',
        phone: '전화',
        actions: '작업',
      },
      editButton: {
        changePassword: '비밀번호 변경',
        edit: '계정 편집',
        addYubikey: 'YubiKey 추가',
        addSSH: 'SSH 키 추가',
        addGPG: 'GPG 키 추가',
        delete: '계정 삭제',
        startEnrollment: '등록 시작',
        activateDesktop: '데스크톱 클라이언트 구성',
        resetPassword: '비밀번호 재설정',
      },
    },
  },
  navigation: {
    bar: {
      overview: 'VPN 개요',
      users: '사용자',
      provisioners: 'YubiKeys',
      webhooks: 'Webhooks',
      openId: 'OpenID 앱',
      myProfile: '내 프로필',
      settings: '설정',
      logOut: '로그아웃',
      enrollment: '등록',
      support: '지원',
      groups: '그룹',
    },
    mobileTitles: {
      groups: '그룹',
      wizard: '위치 생성',
      users: '사용자',
      settings: '설정',
      user: '사용자 프로필',
      provisioners: 'Yubikey',
      webhooks: 'Webhooks',
      openId: 'OpenId 앱',
      overview: '위치 개요',
      networkSettings: '위치 편집',
      enrollment: '등록',
      support: '지원',
    },
    copyright: 'Copyright ©2023-2024',
    version: {
      open: '애플리케이션 버전: {version: string}',
      closed: 'v{version: string}',
    },
  },
  form: {
    download: '다운로드',
    copy: '복사',
    saveChanges: '변경 사항 저장',
    submit: '제출',
    login: '로그인',
    cancel: '취소',
    close: '닫기',
    placeholders: {
      password: '비밀번호',
      username: '사용자 이름',
    },
    error: {
      forbiddenCharacter: '필드에 금지된 문자가 포함되어 있습니다.',
      usernameTaken: '사용자 이름이 이미 사용 중입니다.',
      invalidKey: '키가 유효하지 않습니다.',
      invalid: '필드가 유효하지 않습니다.',
      required: '필드는 필수입니다.',
      invalidCode: '제출된 코드가 유효하지 않습니다.',
      maximumLength: '최대 길이를 초과했습니다.',
      minimumLength: '최소 길이에 도달하지 않았습니다.',
      noSpecialChars: '특수 문자는 허용되지 않습니다.',
      oneDigit: '숫자 하나가 필요합니다.',
      oneSpecial: '특수 문자가 필요합니다.',
      oneUppercase: '대문자 하나가 필요합니다.',
      oneLowercase: '소문자 하나가 필요합니다.',
      portMax: '최대 포트는 65535입니다.',
      endpoint: '유효한 엔드포인트를 입력하세요.',
      address: '유효한 주소를 입력하세요.',
      validPort: '유효한 포트를 입력하세요.',
      validCode: '코드는 6자리여야 합니다.',
      allowedIps: '유효한 IP 또는 도메인만 허용됩니다.',
      startFromNumber: '숫자로 시작할 수 없습니다.',
      repeat: `필드가 일치하지 않습니다.`,
      number: '유효한 숫자를 입력해야 합니다.',
      minimumValue: `{value: number}의 최솟값에 도달하지 않았습니다.`,
      maximumValue: '{value: number}의 최댓값을 초과했습니다.',
      tooManyBadLoginAttempts: `잘못된 로그인 시도가 너무 많습니다. 몇 분 후에 다시 시도하십시오.`,
    },
    floatingErrors: {
      title: '다음을 수정하십시오:',
    },
  },
  components: {
    deviceConfigsCard: {
      cardTitle: '위치에 대한 WireGuard 구성:',
      messages: {
        copyConfig: '클립보드에 구성이 복사되었습니다.',
      },
    },
    gatewaysStatus: {
      label: '게이트웨이',
      states: {
        connected: '모두 연결됨',
        partial: '하나 이상 작동하지 않음',
        disconnected: '연결 끊김',
        error: '연결 정보를 가져오는 데 실패했습니다.',
        loading: '연결 정보를 가져오는 중',
      },
      messages: {
        error: '게이트웨이 상태를 가져오지 못했습니다',
        deleteError: '게이트웨이를 삭제하지 못했습니다',
      },
    },
    noLicenseBox: {
      footer: {
        get: '엔터프라이즈 라이선스 받기',
        contact: '연락처:',
      },
    },
  },
  settingsPage: {
    title: '설정',
    tabs: {
      smtp: 'SMTP',
      global: '전역 설정',
      ldap: 'LDAP',
      openid: 'OpenID',
      enterprise: '엔터프라이즈 기능',
    },
    messages: {
      editSuccess: '설정이 업데이트되었습니다',
      challengeSuccess: '챌린지 메시지가 변경되었습니다',
    },
    enterpriseOnly: {
      title: '이 기능은 Defguard Enterprise에서만 사용할 수 있습니다.',
      subtitle: '자세한 내용은 ',
      website: '웹사이트',
    },
    ldapSettings: {
      title: 'LDAP 설정',
      form: {
        labels: {
          ldap_url: 'URL',
          ldap_bind_username: '바인드 사용자 이름',
          ldap_bind_password: '바인드 비밀번호',
          ldap_member_attr: '멤버 속성',
          ldap_username_attr: '사용자 이름 속성',
          ldap_user_obj_class: '사용자 객체 클래스',
          ldap_user_search_base: '사용자 검색 기준',
          ldap_groupname_attr: '그룹 이름 속성',
          ldap_group_search_base: '그룹 검색 기준',
          ldap_group_member_attr: '그룹 멤버 속성',
          ldap_group_obj_class: '그룹 객체 클래스',
        },
        delete: '구성 삭제',
      },
      test: {
        title: 'LDAP 연결 테스트',
        submit: '테스트',
        messages: {
          success: 'LDAP 연결 성공',
          error: 'LDAP 연결 거부됨',
        },
      },
    },
    openIdSettings: {
      general: {
        title: '외부 OpenID 설정',
        helper: '여기에서 Defguard 인스턴스의 일반 OpenID 동작을 변경할 수 있습니다.',
        createAccount: {
          label:
            '외부 OpenID를 통해 처음 로그인할 때 사용자 계정을 자동으로 생성합니다.',
          helper:
            '이 옵션을 활성화하면 Defguard는 외부 OpenID 공급자를 사용하여 처음 로그인하는 사용자에 대한 새 계정을 자동으로 생성합니다. 그렇지 않으면 관리자가 먼저 사용자 계정을 생성해야 합니다.',
        },
      },
      form: {
        title: '외부 OpenID 클라이언트 설정',
        helper:
          '여기에서 외부 OpenID 공급자가 제공한 값으로 OpenID 클라이언트 설정을 구성할 수 있습니다.',
        custom: '사용자 정의',
        documentation: '설명서',
        delete: '공급자 삭제',
        labels: {
          provider: {
            label: '공급자',
            helper:
              'OpenID 공급자를 선택하세요. 사용자 정의 공급자를 사용하고 직접 기본 URL을 입력할 수 있습니다.',
          },
          client_id: {
            label: '클라이언트 ID',
            helper: 'OpenID 공급자가 제공한 클라이언트 ID입니다.',
          },
          client_secret: {
            label: '클라이언트 보안 비밀',
            helper: 'OpenID 공급자가 제공한 클라이언트 보안 비밀입니다.',
          },
          base_url: {
            label: '기본 URL',
            helper:
              'OpenID 공급자의 기본 URL입니다(예: https://accounts.google.com). 자세한 정보 및 예는 설명서를 확인하십시오.',
          },
        },
      },
    },
    modulesVisibility: {
      header: '모듈 가시성',
      helper: `<p>
            사용하지 않는 모듈이 있는 경우 해당 모듈의 가시성을 비활성화할 수 있습니다.
          </p>
          <a href="{documentationLink:string}" target="_blank">
            자세한 내용은 설명서를 참조하십시오.
          </a>`,
      fields: {
        wireguard_enabled: {
          label: 'WireGuard VPN',
        },
        webhooks_enabled: {
          label: '웹훅',
        },
        worker_enabled: {
          label: 'Yubikey 프로비저닝',
        },
        openid_enabled: {
          label: 'OpenID Connect',
        },
      },
    },
    defaultNetworkSelect: {
      header: '기본 위치 보기',
      helper: `<p>여기에서 기본 위치 보기를 변경할 수 있습니다.</p>
          <a href="{documentationLink:string}" target="_blank">
            자세한 내용은 설명서를 참조하십시오.
          </a>`,
      filterLabels: {
        grid: '그리드 보기',
        list: '목록 보기',
      },
    },
    web3Settings: {
      header: 'Web3 / Wallet connect',
      fields: {
        signMessage: {
          label: '기본 서명 메시지 템플릿',
        },
      },
      controls: {
        save: '변경 사항 저장',
      },
    },
    instanceBranding: {
      header: '인스턴스 브랜딩',
      form: {
        title: '이름 및 로고:',
        fields: {
          instanceName: {
            label: '인스턴스 이름',
            placeholder: 'Defguard',
          },
          mainLogoUrl: {
            label: '로그인 로고 url',
            helper: '<p>최대 사진 크기는 250x100  px입니다</p>',
            placeholder: '기본 이미지',
          },
          navLogoUrl: {
            label: '메뉴 및 탐색 작은 로고',
            helper: '<p>최대 사진 크기는 100x100 px입니다</p>',
            placeholder: '기본 이미지',
          },
        },
        controls: {
          restoreDefault: '기본값 복원',
          submit: '변경 사항 저장',
        },
      },
      helper: `
			      <p>
            여기에서 defguard 인스턴스의 로고 및 이름 url을
            추가할 수 있습니다. defguard 대신 표시됩니다.
          </p>
          <a href="{documentationLink:string}" target="_blank">
            자세한 내용은 설명서를 참조하십시오.
          </a>
			`,
    },
    license: {
      header: '엔터프라이즈',
      helpers: {
        enterpriseHeader: {
          text: '여기에서 Defguard Enterprise 버전 라이선스를 관리할 수 있습니다.',
          link: 'Defguard Enterprise에 대한 자세한 내용은 웹사이트를 방문하십시오.',
        },
        licenseKey: {
          text: '아래에 Defguard Enterprise 라이선스 키를 입력하세요. 라이선스 구매 후 이메일을 통해 받아야 합니다.',
          link: '라이선스는 여기에서 구입할 수 있습니다.',
        },
      },
      form: {
        title: '라이선스',
        fields: {
          key: {
            label: '라이선스 키',
            placeholder: 'Defguard 라이선스 키',
          },
        },
      },
      licenseInfo: {
        title: '라이선스 정보',
        noLicense: '라이선스 없음',
        types: {
          subscription: {
            label: '구독',
            helper: '정기적으로 자동 갱신되는 라이선스',
          },
          offline: {
            label: '오프라인',
            helper:
              '라이선스는 만료 날짜까지 유효하며 자동으로 갱신되지 않습니다',
          },
        },
        fields: {
          type: {
            label: '유형',
          },
          validUntil: {
            label: '유효 기간',
          },
        },
      },
    },
    smtp: {
      form: {
        title: 'SMTP 구성',
        fields: {
          encryption: {
            label: '암호화',
          },
          server: {
            label: '서버 주소',
            placeholder: '주소',
          },
          port: {
            label: '서버 포트',
            placeholder: '포트',
          },
          user: {
            label: '서버 사용자 이름',
            placeholder: '사용자 이름',
          },
          password: {
            label: '서버 비밀번호',
            placeholder: '비밀번호',
          },
          sender: {
            label: '보내는 사람 이메일 주소',
            placeholder: '주소',
            helper: `
              <p>
                시스템 메시지는 이 주소에서 발송됩니다.
                예: no-reply@my-company.com.
              </p>
            `,
          },
        },
        controls: {
          submit: '변경 사항 저장',
        },
      },
      delete: '구성 삭제',
      testForm: {
        title: '테스트 이메일 보내기',
        fields: {
          to: {
            label: '주소',
            placeholder: '주소',
          },
        },
        controls: {
          submit: '보내기',
          success: '테스트 이메일 전송됨',
          error: '이메일 전송 오류',
        },
      },
      helper: `
        <p>
          여기에서 사용자에게 시스템 메시지를 보내는 데 사용되는 SMTP 서버를 구성할 수 있습니다.
        </p>
            `,
    },
    enrollment: {
      helper:
        '등록은 신입 직원이 새 계정을 활성화하고, 비밀번호를 생성하고, VPN 장치를 구성할 수 있도록 하는 프로세스입니다.',
      vpnOptionality: {
        header: 'VPN 단계 선택 사항',
        helper:
          '등록 중 VPN 장치 생성을 선택 사항 또는 필수 사항으로 선택할 수 있습니다.',
      },
      welcomeMessage: {
        header: '환영 메시지',
        helper: `
        <p>이 텍스트 입력란에서는 Markdown을 사용할 수 있습니다:</p>
        <ul>
          <li>제목은 해시 #로 시작합니다</li>
          <li>별표를 사용하여 <i>*이탤릭체*</i>를 만듭니다</li>
          <li>별표 두 개를 사용하여 <b>**굵게**</b> 만듭니다</li>
        </ul>
        `,
      },
      welcomeEmail: {
        header: '환영 이메일',
        helper: `
        <p>이 텍스트 입력란에서는 Markdown을 사용할 수 있습니다:</p>
        <ul>
          <li>제목은 해시 #로 시작합니다</li>
          <li>별표를 사용하여 <i>*이탤릭체*</i>를 만듭니다</li>
          <li>별표 두 개를 사용하여 <b>**굵게**</b> 만듭니다</li>
        </ul>
        `,
      },
      form: {
        controls: {
          submit: '변경 사항 저장',
        },
        welcomeMessage: {
          helper:
            '등록이 완료되면 사용자에게 이 정보가 표시됩니다. 관련 링크를 삽입하고 다음 단계를 간략하게 설명하는 것이 좋습니다.',
          placeholder: '환영 메시지를 입력하세요',
        },
        welcomeEmail: {
          helper:
            '등록이 완료되면 사용자에게 이 정보가 전송됩니다. 관련 링크를 삽입하고 다음 단계를 간략하게 설명하는 것이 좋습니다. 환영 메시지를 여기에서 다시 사용할 수 있습니다.',
          placeholder: '환영 이메일을 입력하세요',
        },
        welcomeEmailSubject: {
          label: '제목',
        },
        useMessageAsEmail: {
          label: '환영 메시지와 동일하게',
        },
      },
    },
    enterprise: {
      header: '엔터프라이즈 기능',
      helper: '<p>여기에서 엔터프라이즈 설정을 변경할 수 있습니다.</p>',
      fields: {
        deviceManagement: {
          label: '사용자가 자신의 장치를 관리하는 기능 비활성화',
          helper:
            "이 옵션을 활성화하면 관리자 그룹의 사용자만 사용자 프로필에서 장치를 관리할 수 있습니다(다른 모든 사용자는 비활성화됨)",
        },
        manualConfig: {
          label: '사용자가 수동 WireGuard 구성을 다운로드하는 기능 비활성화',
          helper:
            "이 옵션을 활성화하면 사용자에게 수동 클라이언트 설정을 위한 WireGuard 구성이 표시되지 않습니다.",
        },
      },
    },
  },
  openidOverview: {
    pageTitle: 'OpenID 앱',
    search: {
      placeholder: '앱 찾기',
    },
    filterLabels: {
      all: '모든 앱',
      enabled: '활성화됨',
      disabled: '비활성화됨',
    },
    clientCount: '모든 앱',
    addNewApp: '새로 추가',
    list: {
      headers: {
        name: '이름',
        status: '상태',
        actions: '작업',
      },
      editButton: {
        edit: '앱 편집',
        delete: '앱 삭제',
        disable: '비활성화',
        enable: '활성화',
        copy: '클라이언트 ID 복사',
      },
      status: {
        enabled: '활성화됨',
        disabled: '비활성화됨',
      },
    },
    messages: {
      copySuccess: '클라이언트 ID가 복사되었습니다.',
      noLicenseMessage: "이 기능에 대한 라이선스가 없습니다.",
      noClientsFound: '결과를 찾을 수 없습니다.',
    },
    deleteApp: {
      title: '앱 삭제',
      message: '{appName: string} 앱을 삭제하시겠습니까?',
      submit: '앱 삭제',
      messages: {
        success: '앱이 삭제되었습니다.',
      },
    },
    enableApp: {
      messages: {
        success: '앱이 활성화되었습니다.',
      },
    },
    disableApp: {
      messages: {
        success: '앱이 비활성화되었습니다.',
      },
    },
    modals: {
      openidClientModal: {
        title: {
          addApp: '애플리케이션 추가',
          editApp: '{appName: string} 앱 편집',
        },
        scopes: '범위:',
        messages: {
          clientIdCopy: '클라이언트 ID 복사됨.',
          clientSecretCopy: '클라이언트 암호 복사됨.',
        },
        form: {
          messages: {
            successAdd: '앱 생성됨.',
            successModify: '앱 수정됨.',
          },
          error: {
            urlRequired: 'URL이 필요합니다.',
            validUrl: '유효한 URL이어야 합니다.',
            scopeValidation: '최소 하나의 범위가 있어야 합니다.',
          },
          fields: {
            name: {
              label: '앱 이름',
            },
            redirectUri: {
              label: '리디렉션 URL {count: number}',
              placeholder: 'https://example.com/redirect',
            },
            openid: {
              label: 'OpenID',
            },
            profile: {
              label: '프로필',
            },
            email: {
              label: '이메일',
            },
            phone: {
              label: '전화',
            },
            groups: {
              label: '그룹',
            },
          },
          controls: {
            addUrl: 'URL 추가',
          },
        },
        clientId: '클라이언트 ID',
        clientSecret: '클라이언트 암호',
      },
    },
  },
  webhooksOverview: {
    pageTitle: 'Webhooks',
    search: {
      placeholder: 'URL로 웹훅 찾기',
    },
    filterLabels: {
      all: '모든 웹훅',
      enabled: '활성화됨',
      disabled: '비활성화됨',
    },
    webhooksCount: '모든 웹훅',
    addNewWebhook: '새로 추가',
    noWebhooksFound: '웹훅을 찾을 수 없습니다.',
    list: {
      headers: {
        name: '이름',
        description: '설명',
        status: '상태',
        actions: '작업',
      },
      editButton: {
        edit: '편집',
        delete: '웹훅 삭제',
        disable: '비활성화',
        enable: '활성화',
      },
      status: {
        enabled: '활성화됨',
        disabled: '비활성화됨',
      },
    },
  },
  provisionersOverview: {
    pageTitle: '프로비저너',
    search: {
      placeholder: '프로비저너 찾기',
    },
    filterLabels: {
      all: '전체',
      available: '사용 가능',
      unavailable: '사용 불가',
    },
    provisionersCount: '모든 프로비저너',
    noProvisionersFound: '프로비저너를 찾을 수 없습니다.',
    noLicenseMessage: "이 기능에 대한 라이선스가 없습니다.",
    provisioningStation: {
      header: 'YubiKey 프로비저닝 스테이션',
      content: `YubiKeys를 프로비저닝하려면 먼저 USB 슬롯이 있는 물리적 시스템을
                설정해야 합니다. 선택한 시스템에서 제공된 명령을 실행하여 등록하고
                키 프로비저닝을 시작하세요.`,
      dockerCard: {
        title: '프로비저닝 스테이션 도커 설정 명령',
      },
      tokenCard: {
        title: '액세스 토큰',
      },
    },
    list: {
      headers: {
        name: '이름',
        ip: 'IP 주소',
        status: '상태',
        actions: '작업',
      },
      editButton: {
        delete: '프로비저너 삭제',
      },
      status: {
        available: '사용 가능',
        unavailable: '사용 불가',
      },
    },
    messages: {
      copy: {
        token: '토큰 복사됨',
        command: '명령 복사됨',
      },
    },
  },
  openidAllow: {
    header: '{name: string}이(가) 다음을 원합니다:',
    scopes: {
      openid: '향후 로그인을 위해 프로필 데이터를 사용합니다.',
      profile: '이름, 프로필 사진 등 프로필의 기본 정보를 알고 있습니다.',
      email: '이메일 주소를 알고 있습니다.',
      phone: '전화번호를 알고 있습니다.',
      groups: '그룹 멤버십을 알고 있습니다.',
    },
    controls: {
      accept: '수락',
      cancel: '취소',
    },
  },
  networkOverview: {
    pageTitle: '위치 개요',
    controls: {
      editNetworks: '위치 설정 편집',
      selectNetwork: {
        placeholder: '위치 로드 중',
      },
    },
    filterLabels: {
      grid: '그리드 보기',
      list: '목록 보기',
    },
    stats: {
      currentlyActiveUsers: '현재 활성 사용자',
      currentlyActiveDevices: '현재 활성 장치',
      activeUsersFilter: '{hour: number}시간 내 활성 사용자',
      activeDevicesFilter: '{hour: number}시간 내 활성 장치',
      totalTransfer: '총 전송량:',
      activityIn: '{hour: number}시간 내 활동',
      in: '들어오는 트래픽:',
      out: '나가는 트래픽:',
      gatewayDisconnected: '게이트웨이 연결 끊김',
    },
  },
  connectedUsersOverview: {
    pageTitle: '연결된 사용자',
    noUsersMessage: '현재 연결된 사용자가 없습니다',
    userList: {
      username: '사용자 이름',
      device: '장치',
      connected: '연결됨',
      deviceLocation: '장치 위치',
      networkUsage: '네트워크 사용량',
    },
  },
  networkPage: {
    pageTitle: '위치 편집',
    addNetwork: '+ 새 위치 추가',
    controls: {
      networkSelect: {
        label: '위치 선택',
      },
    },
  },
  activityOverview: {
    header: '활동 스트림',
    noData: '현재 감지된 활동이 없습니다',
  },
  networkConfiguration: {
    messages: {
      delete: {
        success: '네트워크 삭제됨',
        error: '네트워크 삭제 실패',
      },
    },
    header: '위치 구성',
    importHeader: '위치 가져오기',
    form: {
      helpers: {
        address:
          '이 주소를 기반으로 VPN 네트워크 주소가 정의됩니다. 예: 10.10.10.1/24 (VPN 네트워크는 10.10.10.0/24가 됩니다)',
        gateway: 'VPN 사용자가 연결하는 데 사용되는 게이트웨이 공개 주소',
        dns: 'wireguard 인터페이스가 활성화될 때 쿼리할 DNS 확인자를 지정합니다.',
        allowedIps:
          'VPN 네트워크를 통해 라우팅되어야 하는 주소/마스크 목록입니다.',
        allowedGroups:
          '기본적으로 모든 사용자가 이 위치에 연결할 수 있습니다. 특정 그룹으로 이 위치에 대한 액세스를 제한하려면 아래에서 선택하십시오.',
      },
      messages: {
        networkModified: '위치가 수정되었습니다.',
        networkCreated: '위치가 생성되었습니다',
      },
      fields: {
        name: {
          label: '위치 이름',
        },
        address: {
          label: '게이트웨이 VPN IP 주소 및 넷마스크',
        },
        endpoint: {
          label: '게이트웨이 주소',
        },
        allowedIps: {
          label: '허용된 IP',
        },
        port: {
          label: '게이트웨이 포트',
        },
        dns: {
          label: 'DNS',
        },
        allowedGroups: {
          label: '허용된 그룹',
          placeholder: '모든 그룹',
        },
        mfa_enabled: {
          label: '이 위치에 MFA 필요',
        },
        keepalive_interval: {
          label: 'Keepalive 간격 [초]',
        },
        peer_disconnect_threshold: {
          label: '피어 연결 끊김 임계값 [초]',
        },
      },
      controls: {
        submit: '변경 사항 저장',
        cancel: '개요로 돌아가기',
        delete: '위치 제거',
      },
    },
  },
  gatewaySetup: {
    header: {
      main: '게이트웨이 서버 설정',
      dockerBasedGatewaySetup: `Docker 기반 게이트웨이 설정`,
      fromPackage: `패키지로부터`,
      oneLineInstall: `한 줄 설치`,
    },
    card: {
      title: 'Docker 기반 게이트웨이 설정',
      authToken: `인증 토큰`,
    },
    button: {
      availablePackages: `사용 가능한 패키지`,
    },
    controls: {
      status: '연결 상태 확인',
    },
    messages: {
      runCommand: `Defguard는 vpn 서버에서 wireguard VPN을 제어하기 위해 게이트웨이 노드를 배포해야 합니다.
            자세한 내용은 [문서]({setupGatewayDocs:string})를 참조하십시오.
            게이트웨이 서버를 배포하는 방법에는 여러 가지가 있으며,
            아래는 Docker 기반 예시입니다. 다른 예시는 [문서]({setupGatewayDocs:string})를 참조하십시오.`,
      createNetwork: `게이트웨이 프로세스를 실행하기 전에 네트워크를 생성하십시오.`,
      noConnection: `연결이 설정되지 않았습니다. 제공된 명령을 실행하십시오.`,
      connected: `게이트웨이가 연결되었습니다.`,
      statusError: '게이트웨이 상태를 가져오지 못했습니다',
      oneLineInstall: `한 줄 설치를 수행하는 경우: https://defguard.gitbook.io/defguard/admin-and-features/setting-up-your-instance/one-line-install
          아무 것도 할 필요가 없습니다.`,
      fromPackage: `https://github.com/DefGuard/gateway/releases/latest에서 사용 가능한 패키지를 설치하고 [문서]({setupGatewayDocs:string})에 따라 \`/etc/defguard/gateway.toml\`을 구성하십시오.
          `,
      authToken: `아래 토큰은 게이트웨이 노드를 인증하고 구성하는 데 필요합니다. 이 토큰을 안전하게 보관하고
      [문서]({setupGatewayDocs:string})에 제공된 배포 지침에 따라 게이트웨이 서버를 성공적으로 설정하십시오.
          자세한 내용 및 정확한 단계는 [문서]({setupGatewayDocs:string})를 참조하십시오.`,
      dockerBasedGatewaySetup: `아래는 Docker 기반 예시입니다. 자세한 내용 및 정확한 단계는 [문서]({setupGatewayDocs:string})를 참조하십시오.`,
    },
  },
  loginPage: {
    pageTitle: '자격 증명을 입력하세요',
    callback: {
      return: '로그인으로 돌아가기',
      error: '외부 OpenID 로그인 중 오류가 발생했습니다',
    },
    mfa: {
      title: '이중 인증',
      controls: {
        useAuthenticator: '대신 인증 앱 사용',
        useWallet: '대신 지갑 사용',
        useWebauthn: '대신 보안 키 사용',
        useRecoveryCode: '대신 복구 코드 사용',
        useEmail: '대신 이메일 사용',
      },
      email: {
        header: '이메일로 전송된 코드를 사용하여 진행하십시오.',
        form: {
          labels: {
            code: '코드',
          },
          controls: {
            resendCode: '코드 재전송',
          },
        },
      },
      totp: {
        header: '인증 앱의 코드를 사용하고 버튼을 클릭하여 진행하십시오.',
        form: {
          fields: {
            code: {
              placeholder: '인증 코드 입력',
            },
          },
          controls: {
            submit: '인증 코드 사용',
          },
        },
      },
      recoveryCode: {
        header: '활성 복구 코드 중 하나를 입력하고 버튼을 클릭하여 로그인하십시오.',
        form: {
          fields: {
            code: {
              placeholder: '복구 코드',
            },
          },
          controls: {
            submit: '복구 코드 사용',
          },
        },
      },
      wallet: {
        header:
          '암호화폐 지갑을 사용하여 로그인하려면 지갑 앱 또는 확장 프로그램에서 메시지에 서명하십시오.',
        controls: {
          submit: '지갑 사용',
        },
        messages: {
          walletError: '서명 프로세스 중 지갑 연결이 끊어졌습니다.',
          walletErrorMfa:
            '지갑이 MFA 로그인에 대해 승인되지 않았습니다. 승인된 지갑을 사용하십시오.',
        },
      },
      webauthn: {
        header: '인증할 준비가 되면 아래 버튼을 누르십시오.',
        controls: {
          submit: '보안 키 사용',
        },
        messages: {
          error: '키를 읽지 못했습니다. 다시 시도하십시오.',
        },
      },
    },
  },
  wizard: {
    completed: '위치 설정 완료',
    configuration: {
      successMessage: '위치 생성됨',
    },
    welcome: {
      header: '위치 마법사에 오신 것을 환영합니다!',
      sub: 'VPN을 사용하기 전에 먼저 위치를 설정해야 합니다. 확실하지 않은 경우 <React> 아이콘을 클릭하십시오.',
      button: '위치 설정',
    },
    navigation: {
      top: '위치 설정',
      titles: {
        welcome: '위치 설정',
        choseNetworkSetup: '위치 설정 선택',
        importConfig: '기존 위치 가져오기',
        manualConfig: '위치 구성',
        mapDevices: '가져온 장치 매핑',
      },
      buttons: {
        next: '다음',
        back: '뒤로',
      },
    },
    deviceMap: {
      messages: {
        crateSuccess: '장치 추가됨',
        errorsInForm: '표시된 필드를 채워주세요.',
      },
      list: {
        headers: {
          deviceName: '장치 이름',
          deviceIP: 'IP',
          user: '사용자',
        },
      },
    },
    wizardType: {
      manual: {
        title: '수동 구성',
        description: '수동 위치 구성',
      },
      import: {
        title: '파일에서 가져오기',
        description: 'WireGuard 구성 파일에서 가져오기',
      },
      createNetwork: '위치 생성',
    },
    common: {
      select: '선택',
    },
    locations: {
      form: {
        name: '이름',
        ip: 'IP 주소',
        user: '사용자',
        fileName: '파일',
        selectFile: '파일 선택',
        messages: { devicesCreated: '장치 생성됨' },
        validation: { invalidAddress: '잘못된 주소' },
      },
    },
  },
  layout: {
    select: {
      addNewOptionDefault: '새로 추가 +',
    },
  },
  redirectPage: {
    title: '로그인되었습니다',
    subtitle: '잠시 후 리디렉션됩니다...',
  },
  enrollmentPage: {
    title: '등록',
    controls: {
      default: '기본값 복원',
      save: '변경 사항 저장',
    },
    messages: {
      edit: {
        success: '설정이 변경되었습니다',
        error: '저장 실패',
      },
    },
    messageBox:
      '등록은 신입 직원이 새 계정을 확인하고, 비밀번호를 생성하고, VPN 장치를 구성할 수 있도록 하는 프로세스입니다. 이 패널에서 관련 메시지를 사용자 지정할 수 있습니다.',
    settings: {
      welcomeMessage: {
        title: '환영 메시지',
        messageBox:
          '이 정보는 등록이 완료되면 서비스 내 사용자에게 표시됩니다. 링크를 삽입하고 다음 단계를 간략하게 설명하는 것이 좋습니다. 이메일에 있는 것과 동일한 메시지를 사용할 수 있습니다.',
      },
      vpnOptionality: {
        title: 'VPN 설정 선택 사항',
        select: {
          options: {
            optional: '선택 사항',
            mandatory: '필수',
          },
        },
      },
      welcomeEmail: {
        title: '환영 이메일',
        subject: {
          label: '이메일 제목',
        },
        messageBox:
          '등록이 완료되면 사용자에게 이 정보가 전송됩니다. 관련 링크를 삽입하고 다음 단계를 간략하게 설명하는 것이 좋습니다.',
        controls: {
          duplicateWelcome: '환영 메시지와 동일',
        },
      },
    },
  },
  supportPage: {
    title: '지원',
    modals: {
      confirmDataSend: {
        title: '지원 데이터 보내기',
        subTitle:
          '실제로 지원 디버그 정보를 보내려는 것인지 확인하십시오. 개인 정보는 전송되지 않습니다(wireguard 키, 이메일 주소 등은 전송되지 않음).',
        submit: '지원 데이터 보내기',
      },
    },
    debugDataCard: {
      title: '지원 데이터',
      body: `
지원이 필요하거나 저희 팀(예: Matrix 지원 채널: **#defguard-support:teonite.com**)에서 지원 데이터 생성을 요청받은 경우 다음 두 가지 옵션이 있습니다.
* SMTP 설정을 구성하고 "지원 데이터 보내기"를 클릭합니다.
* 또는 "지원 데이터 다운로드"를 클릭하고 이 파일을 첨부하여 GitHub에 버그 보고서를 생성합니다.
`,
      downloadSupportData: '지원 데이터 다운로드',
      downloadLogs: '로그 다운로드',
      sendMail: '지원 데이터 보내기',
      mailSent: '이메일 전송됨',
      mailError: '이메일 전송 오류',
    },
    supportCard: {
      title: '지원',
      body: `
GitHub에 문의하거나 문제를 제출하기 전에 [defguard.gitbook.io/defguard](https://defguard.gitbook.io/defguard/)에서 제공되는 Defguard 문서를 숙지하십시오.

제출하려면:
* 버그 - [GitHub](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=bug&template=bug_report.md&title=)로 이동하십시오.
* 기능 요청 - [GitHub](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=feature&template=feature_request.md&title=)로 이동하십시오.

기타 요청은 support@defguard.net으로 문의하십시오.
`,
    },
  },
};

export default ko;

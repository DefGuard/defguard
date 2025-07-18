import type { BaseTranslation } from '../i18n-types';

const en: BaseTranslation = {
  common: {
    conditions: {
      or: 'or',
      and: 'and',
      equal: 'equal',
    },
    controls: {
      timeRange: 'Time range',
      addNew: 'Add new',
      add: 'Add',
      accept: 'Accept',
      next: 'Next',
      back: 'Back',
      cancel: 'Cancel',
      confirm: 'Confirm',
      submit: 'Submit',
      close: 'Close',
      select: 'Select',
      finish: 'Finish',
      saveChanges: 'Save changes',
      save: 'Save',
      RestoreDefault: 'Restore default',
      delete: 'Delete',
      rename: 'Rename',
      copy: 'Copy',
      edit: 'Edit',
      dismiss: 'Dismiss',
      show: 'Show',
      enable: 'Enable',
      enabled: 'Enabled',
      disable: 'Disable',
      disabled: 'Disabled',
      selectAll: 'Select all',
      clear: 'Clear',
      clearAll: 'Clear all',
      filter: 'Filter',
      filters: 'Filters',
    },
    key: 'Key',
    name: 'Name',
    noData: 'No data',
    unavailable: 'Unavailable',
    notSet: 'Not set',
    search: 'Search',
    time: 'Time',
    from: 'From',
    until: 'Until',
  },
  messages: {
    error: 'Error has occurred.',
    success: 'Operation succeeded',
    errorVersion: 'Failed to get application version.',
    insecureContext: 'Context is not secure.',
    details: 'Details:',
    clipboard: {
      error: 'Clipboard is not accessible.',
      success: 'Content copied to clipboard.',
    },
  },
  modals: {
    upgradeLicenseModal: {
      enterprise: {
        title: 'Upgrade to Enterprise',
        //md
        subTitle: `This functionality is an **enterprise feature** and you've exceeded the user, device or network limits to use it. In order to use this feature, purchase an enterprise license or upgrade your existing one.`,
      },
      limit: {
        title: 'Upgrade',
        //md
        subTitle: `
        You have **reached the limit** of this functionality. To **[ manage more locations/users/devices ]** purchase of the Enterprise license is required.
        `,
      },
      //md
      content: `
You can find out more about features like:
- Real time and automatic client synchronization
- External SSO
- Controlling VPN clients behavior

Full enterprise feature list: [https://docs.defguard.net/enterprise/all-enteprise-features](https://docs.defguard.net/enterprise/all-enteprise-features)</br>
Licensing information: [https://docs.defguard.net/enterprise/license](https://docs.defguard.net/enterprise/license)
      `,
      controls: {
        cancel: 'Maybe later',
        confirm: 'See all Enterprise plans',
      },
    },
    standaloneDeviceEnrollmentModal: {
      title: 'Network device token',
      toasters: {
        error: 'Token generation failed.',
      },
    },
    standaloneDeviceConfigModal: {
      title: 'Network device config',
      cardTitle: 'Config',
      toasters: {
        getConfig: {
          error: 'Failed to get device config.',
        },
      },
    },
    editStandaloneModal: {
      title: 'Edit network device',
      toasts: {
        success: 'Device modified',
        failure: 'Modifying the device failed',
      },
    },
    deleteStandaloneDevice: {
      title: 'Delete network device',
      content: 'Device {name: string} will be deleted.',
      messages: {
        success: 'Device deleted',
        error: 'Failed to remove device.',
      },
    },
    addStandaloneDevice: {
      toasts: {
        deviceCreated: 'Device added',
        creationFailed: 'Device could not be added.',
      },
      infoBox: {
        setup:
          'Here you can add definitions or generate configurations for devices that can connect to your VPN. Only locations without Multi-Factor Authentication are available here, as MFA is only supported in Defguard Desktop Client for now.',
      },
      form: {
        submit: 'Add Device',
        labels: {
          deviceName: 'Device Name',
          location: 'Location',
          assignedAddress: 'Assigned IP Address',
          description: 'Description',
          generation: {
            auto: 'Generate key pair',
            manual: 'Use my own public key',
          },
          publicKey: 'Provide Your Public Key',
        },
      },
      steps: {
        method: {
          title: 'Choose a preferred method',
          cards: {
            cli: {
              title: 'Defguard Command Line Client',
              subtitle:
                'When using defguard-cli your device will be automatically configured.',
              docs: 'Defguard CLI download and documentation',
            },
            manual: {
              title: 'Manual WireGuard Client',
              subtitle:
                'If your device does not support our CLI binaries you can always generate a WireGuard configuration file and configure it manually - but any updates to the VPN location configuration will require manual changes in device configuration.',
            },
          },
        },
        manual: {
          title: 'Add new VPN device using WireGuard Client',
          finish: {
            messageTop:
              'Download the provided configuration file to your device and import it into your VPN client to complete the setup.',
            ctaInstruction:
              "Use provided configuration file below by scanning QR code or importing it as file on your device's WireGuard app.",
            // MD
            warningMessage: `
            Please remember that Defguard **doesn't store private keys**. We will securely generate the public and private key pair in your browser, but only store the public key in Defguard database. Please download the configuration generated with the private key for the device, as it will not be accessible later.
            `,
            actionCard: {
              title: 'Config',
            },
          },
        },
        cli: {
          title: 'Add device using Defguard Command Line Client',
          finish: {
            topMessage:
              'First download Defguard command line client binary and install it on your server.',
            downloadButton: 'Download Defguard CLI Client',
            commandCopy: 'Copy and paste this command in your terminal on the device',
          },
          setup: {
            stepMessage:
              'Here you can add definitions or generate configurations for devices that can connect to your VPN. Only locations without Multi-Factor Authentication are available here, as MFA is only supported in Defguard Desktop Client for now.',
            form: {
              submit: 'Add Device',
            },
          },
        },
      },
    },
    updatesNotificationToaster: {
      title: 'New version available {version: string}',
      controls: {
        more: "See what's new",
      },
    },
    enterpriseUpgradeToaster: {
      title: `You've reached the enterprise functionality limit.`,
      message: `You've exceeded the limit of your current Defguard plan and the enterprise
          features will be disabled. Purchase an enterprise license or upgrade your
          existing one to continue using these features.`,
      link: 'See all enterprise plans',
    },
    updatesNotification: {
      header: {
        title: 'Update Available',
        newVersion: 'new version {version: string}',
        criticalBadge: 'critical update',
      },
      controls: {
        visitRelease: 'Visit release page',
      },
    },
    addGroup: {
      title: 'Add group',
      selectAll: 'Select all users',
      groupName: 'Group name',
      searchPlaceholder: 'Filter/Search',
      submit: 'Create group',
      groupSettings: 'Group settings',
      adminGroup: 'Admin group',
    },
    editGroup: {
      title: 'Edit group',
      selectAll: 'Select all users',
      groupName: 'Group name',
      searchPlaceholder: 'Filter/Search',
      submit: 'Update group',
      groupSettings: 'Group settings',
      adminGroup: 'Admin group',
    },
    deleteGroup: {
      title: 'Delete group {name:string}',
      subTitle: 'This action will permanently delete this group.',
      locationListHeader: 'This group is currently assigned to following VPN Locations:',
      locationListFooter: `If this is the only allowed group for a given location, the location will become <b>accessible to all users</b>.`,
      submit: 'Delete group',
      cancel: 'Cancel',
    },
    deviceConfig: {
      title: 'Device VPN configurations',
    },
    changePasswordSelf: {
      title: 'Change password',
      messages: {
        success: 'Password has been changed',
        error: 'Failed to changed password',
      },
      form: {
        labels: {
          newPassword: 'New password',
          oldPassword: 'Current password',
          repeat: 'Confirm new password',
        },
      },
      controls: {
        submit: 'Change password',
        cancel: 'Cancel',
      },
    },
    disableMfa: {
      title: 'Disable MFA',
      message: 'Do you want to disable MFA for user {username: string}?',
      messages: {
        success: 'MFA for user {username: string} has been disabled',
        error: 'Failed to disable MFA for user {username: string}',
      },
      controls: {
        submit: 'Disable MFA',
        cancel: 'Cancel',
      },
    },
    startEnrollment: {
      title: 'Start enrollment',
      desktopTitle: 'Desktop activation',
      messages: {
        success: 'User enrollment started',
        successDesktop: 'Desktop configuration started',
        error: 'Failed to start user enrollment',
        errorDesktop: 'Failed to start desktop activation',
      },
      form: {
        email: {
          label: 'Email',
        },
        mode: {
          options: {
            email: 'Send token by email',
            manual: 'Deliver token yourself',
          },
        },
        submit: 'Start enrollment',
        submitDesktop: 'Activate desktop',
        smtpDisabled: 'Configure SMTP to send token by email. Go to Settings -> SMTP.',
      },
      tokenCard: {
        title: 'Activation token',
      },
      urlCard: {
        title: 'Defguard Instance URL',
      },
    },
    deleteNetwork: {
      title: 'Delete {name:string} location',
      subTitle: 'This action will permanently delete this location.',
      submit: 'Delete location',
      cancel: 'Cancel',
    },
    changeWebhook: {
      messages: {
        success: 'Webhook changed.',
      },
    },
    manageWebAuthNKeys: {
      title: 'Security keys',
      messages: {
        deleted: 'WebAuthN key has been deleted.',
        duplicateKeyError: 'Key is already registered',
      },
      infoMessage: `
        <p>
          Security keys can be used as your second factor of authentication
          instead of a verification code. Learn more about configuring a
          security key.
        </p>
`,
      form: {
        messages: {
          success: 'Security key added.',
        },
        fields: {
          name: {
            label: 'New key name',
          },
        },
        controls: {
          submit: 'Add new Key',
        },
      },
    },
    recoveryCodes: {
      title: 'Recovery codes',
      submit: 'I have saved my codes',
      messages: {
        copied: 'Codes copied.',
      },
      infoMessage: `
        <p>
          Treat your recovery codes with the same level of attention as you
          would your password! We recommend saving them with a password manager
          such as Lastpass, bitwarden or Keeper.
        </p>
`,
    },
    registerTOTP: {
      title: 'Authenticator App Setup',
      infoMessage: `
        <p>
          To setup your MFA, scan this QR code with your authenticator app, then
          enter the code in the field below:
        </p>
`,
      messages: {
        totpCopied: 'TOTP path copied.',
        success: 'TOTP Enabled',
      },
      copyPath: 'Copy TOTP path',
      form: {
        fields: {
          code: {
            label: 'Authenticator code',
            error: 'Code is invalid',
          },
        },
        controls: {
          submit: 'Verify code',
        },
      },
    },
    registerEmailMFA: {
      title: 'Email MFA Setup',
      infoMessage: `
        <p>
          To setup your MFA enter the code that was sent to your account email: <strong>{email: string}</strong>
        </p>
`,
      messages: {
        success: 'Email MFA Enabled',
        resend: 'Verification code resent',
      },
      form: {
        fields: {
          code: {
            label: 'Email code',
            error: 'Code is invalid',
          },
        },
        controls: {
          submit: 'Verify code',
          resend: 'Resend email',
        },
      },
    },
    editDevice: {
      title: 'Edit device',
      messages: {
        success: 'Device has been updated.',
      },
      form: {
        fields: {
          name: {
            label: 'Device Name',
          },
          publicKey: {
            label: 'Device Public Key (WireGuard)',
          },
        },
        controls: {
          submit: 'Edit device',
        },
      },
    },
    deleteDevice: {
      title: 'Delete device',
      message: 'Do you want to delete {deviceName} device ?',
      submit: 'Delete device',
      messages: {
        success: 'Device has been deleted.',
      },
    },
    keyDetails: {
      title: 'YubiKey details',
      downloadAll: 'Download all keys',
    },
    deleteUser: {
      title: 'Delete account',
      controls: {
        submit: 'Delete account',
      },
      message: 'Do you want to delete {username: string} account permanently?',
      messages: {
        success: '{username: string} deleted.',
      },
    },
    disableUser: {
      title: 'Disable account',
      controls: {
        submit: 'Disable account',
      },
      message: 'Do you want to disable {username: string} account?',
      messages: {
        success: '{username: string} disabled.',
      },
    },
    enableUser: {
      title: 'Enable account',
      controls: {
        submit: 'Enable account',
      },
      message: 'Do you want to enable {username: string} account?',
      messages: {
        success: '{username: string} enabled.',
      },
    },
    deleteProvisioner: {
      title: 'Delete provisioner',
      controls: {
        submit: 'Delete provisioner',
      },
      message: 'Do you want to delete {id: string} provisioner?',
      messages: {
        success: '{provisioner: string} deleted.',
      },
    },
    changeUserPassword: {
      messages: {
        success: 'Password changed.',
      },
      title: 'Change user password',
      form: {
        controls: {
          submit: 'Save new password',
        },
        fields: {
          newPassword: {
            label: 'New password',
          },
          confirmPassword: {
            label: 'Repeat password',
          },
        },
      },
    },
    provisionKeys: {
      title: 'Yubikey provisioning:',
      warning:
        'Please be advised that this operation wll wipe openpgp application on yubikey and reconfigure it.',
      infoBox: `The selected provisioner must have a <b>clean</b> YubiKey
                plugged in be provisioned. To clean a used YubiKey
                <b>gpg --card-edit </b> before provisioning.`,
      selectionLabel: 'Select one of the following provisioners to provision a YubiKey:',
      noData: {
        workers: 'No workers found, waiting...',
      },
      controls: {
        submit: 'Provision YubiKey',
      },
      messages: {
        success: 'Keys provisioned',
        errorStatus: 'Error while getting worker status.',
      },
    },
    addUser: {
      title: 'Add new user',
      messages: {
        userAdded: 'User added',
      },
      form: {
        submit: 'Add user',
        fields: {
          username: {
            placeholder: 'login',
            label: 'Login',
          },
          password: {
            placeholder: 'Password',
            label: 'Password',
          },
          email: {
            placeholder: 'User e-mail',
            label: 'User e-mail',
          },
          firstName: {
            placeholder: 'First name',
            label: 'First name',
          },
          lastName: {
            placeholder: 'Last name',
            label: 'Last name',
          },
          phone: {
            placeholder: 'Phone',
            label: 'Phone',
          },
          enableEnrollment: {
            label: 'Use user self-enrollment process',
            link: '<a href="https://docs.defguard.net/help/enrollment" target="_blank">more information here</a>',
          },
        },
      },
    },
    webhookModal: {
      title: {
        addWebhook: 'Add webhook.',
        editWebhook: 'Edit webhook',
      },
      messages: {
        clientIdCopy: 'Client ID copied.',
        clientSecretCopy: 'Client secret copied.',
      },
      form: {
        triggers: 'Trigger events:',
        messages: {
          successAdd: 'Webhook created.',
          successModify: 'Webhook modified.',
        },
        error: {
          urlRequired: 'URL is required.',
          validUrl: 'Must be a valid URL.',
          scopeValidation: 'Must have at least one trigger.',
          tokenRequired: 'Token is required.',
        },
        fields: {
          description: {
            label: 'Description',
            placeholder: 'Webhook to create gmail account on new user',
          },
          token: {
            label: 'Secret token',
            placeholder: 'Authorization token',
          },
          url: {
            label: 'Webhook URL',
            placeholder: 'https://example.com/webhook',
          },
          userCreated: {
            label: 'New user Created',
          },
          userDeleted: {
            label: 'User deleted',
          },
          userModified: {
            label: 'User modified',
          },
          hwkeyProvision: {
            label: 'User Yubikey provision',
          },
        },
      },
    },
    deleteWebhook: {
      title: 'Delete webhook',
      message: 'Do you want to delete {name: string} webhook ?',
      submit: 'Delete',
      messages: {
        success: 'Webhook deleted.',
      },
    },
  },
  addDevicePage: {
    title: 'Add device',
    helpers: {
      setupOpt: `You can add a device using this wizard. Opt for our native application "defguard" or any other WireGuard client. If you're unsure, we recommend using defguard for simplicity.`,
      client: `Please download defguard desktop client <a href="https://defguard.net/download" target="_blank">here</a> and then follow <a href="https://docs.defguard.net/help/configuring-vpn/add-new-instance" target="_blank">this guide</a>.`,
    },
    messages: {
      deviceAdded: 'Device added',
    },
    steps: {
      setupMethod: {
        title: 'Choose Your Connection Method',
        message:
          "You can add a device using this wizard. To proceed, you'll need to install the defguard Client on the device you're adding. You can also use any standard WireGuard® client, but for the best experience and ease of setup, we recommend using our native defguard Client.",
        methods: {
          client: {
            title: 'Remote Device Activation',
            description:
              'Use the Defguard Client to set up your device. Easily configure it with a single token or by scanning a QR code.',
          },
          wg: {
            title: 'Manual WireGuard Client',
            description:
              'For advanced users, get a unique config via download or QR code. Download any WireGuard® client and take control of your VPN setup.',
          },
        },
      },
      client: {
        title: 'Client Activation',
        message:
          'Please enter the provided Instance URL and Token into your Defguard Client. You can scan the QR code or copy and paste the token manually.',
        qrDescription:
          "Scan the QR code with your installed Defguard app. If you haven't installed it yet, use your device's app store or the link below.",
        desktopDownload: 'Download defguard Client for desktop device',
        tokenCopy: 'Token copied to clipboard',
        tokenFailure: 'Failed to prepare client setup',
        labels: {
          mergedToken: 'Defguard Instance Token (new)',
          token: 'Authentication Token',
          url: 'URL',
        },
      },
      configDevice: {
        title: 'Configure device',
        messages: {
          copyConfig: 'Configuration has been copied to the clipboard',
        },
        helpers: {
          warningAutoMode: `
    <p>
      Please be advised that you have to download the configuration now,
      since <strong>we do not</strong> store your private key. After this
      page is closed, you <strong>will not be able</strong> to get your
      full configuration file (with private keys, only blank template).
    </p>
`,
          warningManualMode: `
    <p>
      Please be advised that configuration provided here <strong> does not include private key and uses public key to fill it's place </strong> you will need to replace it on your own for configuration to work properly.
    </p>
`,
          warningNoNetworks: "You don't have access to any network.",
          qrHelper: `
      <p>
        You can setup your device faster with wireguard application by scanning this QR code.
      </p>`,
        },
        qrInfo:
          'Use provided configuration file below by scanning QR Code or importing it as file on your devices WireGuard instance.',
        inputNameLabel: 'Device Name',
        qrLabel: 'WireGuard Config File',
      },
      setupDevice: {
        title: 'Create VPN device',
        infoMessage: `
        <p>
          You need to configure WireGuardVPN on your device, please visit&nbsp;
          <a href="{addDevicesDocs:string}">documentation</a> if you don&apos;t know how to do it.
        </p>
`,
        options: {
          auto: 'Generate key pair',
          manual: 'Use my own public key',
        },
        form: {
          fields: {
            name: {
              label: 'Device Name',
            },
            publicKey: {
              label: 'Provide Your Public Key',
            },
          },
          errors: {
            name: {
              duplicatedName: 'Device with this name already exists',
            },
          },
        },
      },
      copyToken: {
        title: 'Client activation',
        tokenCardTitle: 'Activation token',
        urlCardTitle: 'Defguard Instance URL',
      },
    },
  },
  userPage: {
    title: {
      view: 'User Profile',
      edit: 'Edit User Profile',
    },
    messages: {
      editSuccess: 'User updated.',
      failedToFetchUserData: 'Could not get user information.',
      passwordResetEmailSent: 'Password reset email has been sent.',
    },
    userDetails: {
      header: 'Profile Details',
      messages: {
        deleteApp: 'App and all tokens deleted.',
      },
      warningModals: {
        title: 'Warning',
        content: {
          usernameChange: `Changing the username has a significant impact on services the user has logged into using Defguard. After changing it, the user may lose access to applications (since they will not recognize them). Are you sure you want to proceed?`,
          emailChange: `If you are using external OpenID Connect (OIDC) providers to authenticate users, changing a user's email address may have a significant impact on their ability to log in to Defguard. Are you sure you want to proceed?`,
        },
        buttons: {
          proceed: 'Proceed',
          cancel: 'Cancel',
        },
      },
      fields: {
        username: {
          label: 'Username',
        },
        firstName: {
          label: 'First name',
        },
        lastName: {
          label: 'Last name',
        },
        phone: {
          label: 'Phone number',
        },
        email: {
          label: 'E-mail',
        },
        status: {
          label: 'Status',
          active: 'Active',
          disabled: 'Disabled',
        },
        groups: {
          label: 'User groups',
          noData: 'No groups',
        },
        apps: {
          label: 'Authorized apps',
          noData: 'No authorized apps',
        },
      },
    },
    userAuthInfo: {
      header: 'Password and authentication',
      password: {
        header: 'Password settings',
        changePassword: 'Change password',
        ldap_change_heading: '{ldapName:string} password update required',
        ldap_change_message:
          "Defguard doesn't store your password in plain text, so we can’t retrieve it for automatic synchronization with your {ldapName:string} credentials. To enable {ldapName:string} login to other services, please update your Defguard password for your {ldapName:string} password to be set — you can re-enter your current password if you wish. This step is necessary to ensure consistent and secure authentication across both systems.",
      },
      recovery: {
        header: 'Recovery options',
        codes: {
          label: 'Recovery Codes',
          viewed: 'Viewed',
        },
      },
      mfa: {
        header: 'Two-factor methods',
        edit: {
          disable: 'Disable MFA',
        },
        messages: {
          mfaDisabled: 'MFA disabled.',
          OTPDisabled: 'One time password disabled.',
          EmailMFADisabled: 'Email MFA disabled.',
          changeMFAMethod: 'MFA method changed',
        },
        securityKey: {
          singular: 'security key',
          plural: 'security keys',
        },
        default: 'default',
        enabled: 'Enabled',
        disabled: 'Disabled',
        labels: {
          totp: 'Time based one time passwords',
          email: 'Email',
          webauth: 'Security keys',
        },
        editMode: {
          enable: 'Enable',
          disable: 'Disable',
          makeDefault: 'Make default',
          webauth: {
            manage: 'Manage security keys',
          },
        },
      },
    },
    controls: {
      editButton: 'Edit profile',
      deleteAccount: 'Delete account',
    },
    devices: {
      header: 'User devices',
      addDevice: {
        web: 'Add new device',
        desktop: 'Add this device',
      },
      card: {
        labels: {
          publicIP: 'Public IP',
          connectedThrough: 'Connected through',
          connectionDate: 'Connected date',
          lastLocation: 'Last connected from',
          lastConnected: 'Last connected',
          assignedIp: 'Assigned IP',
          active: 'active',
          noData: 'Never connected',
        },
        edit: {
          edit: 'Edit device',
          delete: 'Delete device',
          showConfigurations: 'Show configuration',
        },
      },
    },
    yubiKey: {
      header: 'User YubiKey',
      provision: 'Provision a YubiKey',
      keys: {
        pgp: 'PGP key',
        ssh: 'SSH key',
      },
      noLicense: {
        moduleName: 'YubiKey module',
        line1: 'This is enterprise module for YubiKey',
        line2: 'management and provisioning.',
      },
    },
    authenticationKeys: {
      header: 'User Authentication Keys',
      addKey: 'Add new Key',
      keysList: {
        common: {
          rename: 'Rename',
          key: 'Key',
          download: 'Download',
          copy: 'Copy',
          serialNumber: 'Serial Number',
          delete: 'Delete',
        },
      },
      deleteModal: {
        title: 'Delete Authentication Key',
        confirmMessage: 'Key {name: string} will be deleted permanently.',
      },
      addModal: {
        header: 'Add new Authentication Key',
        keyType: 'Key Type',
        keyForm: {
          placeholders: {
            title: 'Key Name',
            key: {
              ssh: 'Begins with ssh-rsa, ecdsa-sha2-nistp256, ...',
              gpg: 'Begins with -----BEGIN PGP PUBLIC KEY BLOCK-----',
            },
          },
          labels: {
            title: 'Name',
            key: 'Key',
          },
          submit: 'Add {name: string} key',
        },
        yubikeyForm: {
          selectWorker: {
            info: 'Please be advised that this operation will wipe openpgp application on YubiKey and reconfigure it.',
            selectLabel: 'Select on of the following provisioners to provision a YubiKey',
            noData: 'No workers are registered right now.',
            available: 'Available',
            unavailable: 'Unavailable',
          },
          provisioning: {
            inProgress: 'Provisioning in progress, please wait.',
            error: 'Provisioning failed !',
            success: 'Yubikey provisioned successfully',
          },
          submit: 'Provision Yubikey',
        },
        messages: {
          keyAdded: 'Key added.',
          keyExists: 'Key has already been added.',
          unsupportedKeyFormat: 'Unsupported key format.',
          genericError: 'Could not add the key. Please try again later.',
        },
      },
    },
    apiTokens: {
      header: 'User API Tokens',
      addToken: 'Add new API Token',
      tokensList: {
        common: {
          rename: 'Rename',
          token: 'Token',
          copy: 'Copy',
          delete: 'Delete',
          createdAt: 'Created at',
        },
      },
      deleteModal: {
        title: 'Delete API Token',
        confirmMessage: 'API token {name: string} will be deleted permanently.',
      },
      addModal: {
        header: 'Add new API Token',
        tokenForm: {
          placeholders: {
            name: 'API Token Name',
          },
          labels: {
            name: 'Name',
          },
          submit: 'Add API token',
        },
        copyToken: {
          warningMessage:
            "Please copy the API token below now. You won't be able to see it again.",
          header: 'Copy new API Token',
        },
        messages: {
          tokenAdded: 'API token added.',
          genericError: 'Could not add API token. Please try again later.',
        },
      },
    },
  },
  usersOverview: {
    pageTitle: 'Users',
    grid: {
      usersTitle: 'Connected Users',
      devicesTitle: 'Connected Network Devices',
    },
    search: {
      placeholder: 'Find users',
    },
    filterLabels: {
      all: 'All users',
      admin: 'Admins only',
      users: 'Users only',
    },
    usersCount: 'All users',
    addNewUser: 'Add new',
    list: {
      headers: {
        name: 'User name',
        username: 'Login',
        phone: 'Phone',
        actions: 'Actions',
      },
      editButton: {
        changePassword: 'Change password',
        edit: 'Edit account',
        addYubikey: 'Add YubiKey',
        addSSH: 'Add SSH Key',
        addGPG: 'Add GPG Key',
        delete: 'Delete account',
        startEnrollment: 'Start enrollment',
        activateDesktop: 'Configure Desktop Client',
        resetPassword: 'Reset password',
        disableMfa: 'Disable MFA',
      },
    },
  },
  navigation: {
    bar: {
      overview: 'VPN Overview',
      users: 'Users',
      provisioners: 'YubiKeys',
      webhooks: 'Webhooks',
      openId: 'OpenID Apps',
      myProfile: 'My Profile',
      settings: 'Settings',
      logOut: 'Log out',
      enrollment: 'Enrollment',
      support: 'Support',
      groups: 'Groups',
      devices: 'Network Devices',
      acl: 'Access Control',
      activity: 'Activity log',
    },
    mobileTitles: {
      activity: 'Activity log',
      groups: 'Groups',
      wizard: 'Create location',
      users: 'Users',
      settings: 'Settings',
      user: 'User Profile',
      provisioners: 'Yubikey',
      webhooks: 'Webhooks',
      openId: 'OpenId Apps',
      overview: 'Location Overview',
      networkSettings: 'Edit Location',
      enrollment: 'Enrollment',
      support: 'Support',
      devices: 'Network Devices',
    },
    copyright: 'Copyright ©2023-2025',
    version: {
      open: 'Application version: {version: string}',
      closed: 'v{version: string}',
    },
  },
  form: {
    download: 'Download',
    copy: 'Copy',
    saveChanges: 'Save changes',
    submit: 'Submit',
    login: 'Sign in',
    cancel: 'Cancel',
    close: 'Close',
    placeholders: {
      password: 'Password',
      username: 'Username',
      username_or_email: 'Username or email',
    },
    error: {
      urlInvalid: 'Enter valid URL',
      reservedName: 'Name is already taken.',
      invalidIp: 'IP is invalid.',
      reservedIp: 'IP is already in use.',
      forbiddenCharacter: 'Field contains forbidden characters.',
      usernameTaken: 'Username is already in use.',
      invalidKey: 'Key is invalid.',
      invalid: 'Field is invalid.',
      required: 'Field is required.',
      invalidCode: 'Submitted code is invalid.',
      maximumLength: 'Maximum length exceeded.',
      maximumLengthOf: `Field length cannot exceed {length: number}`,
      minimumLength: 'Minimum length not reached.',
      minimumLengthOf: `Minimum length of {length: number} not reached.`,
      noSpecialChars: 'No special characters are allowed.',
      oneDigit: 'One digit required.',
      oneSpecial: 'Special character required.',
      oneUppercase: 'One uppercase character required.',
      oneLowercase: 'One lowercase character required.',
      portMax: 'Maximum port is 65535.',
      endpoint: 'Enter a valid endpoint.',
      address: 'Enter a valid address.',
      addressNetmask: 'Enter a valid address with a netmask.',
      validPort: 'Enter a valid port.',
      validCode: 'Code should have 6 digits.',
      allowedIps: 'Only valid IP or domain is allowed.',
      startFromNumber: 'Cannot start from number.',
      repeat: `Fields don't match.`,
      number: 'Expected a valid number.',
      minimumValue: `Minimum value of {value: number} not reached.`,
      maximumValue: 'Maximum value of {value: number} exceeded.',
      tooManyBadLoginAttempts: `Too many bad login attempts. Please try again in a few minutes.`,
    },
    floatingErrors: {
      title: 'Please correct the following:',
    },
  },
  components: {
    aclDefaultPolicySelect: {
      label: 'Default ACL Policy',
      options: {
        allow: 'Allow',
        deny: 'Deny',
      },
    },
    standaloneDeviceTokenModalContent: {
      headerMessage:
        'First download defguard command line client binaries and install them on your server.',
      downloadButton: 'Download Defguard CLI Client',
      expandableCard: {
        title: 'Copy and paste this command in your terminal on the device',
      },
    },
    deviceConfigsCard: {
      cardTitle: 'WireGuard Config for location:',
      messages: {
        copyConfig: 'Configuration copied to the clipboard',
      },
    },
    gatewaysStatus: {
      label: 'Gateways',
      states: {
        all: 'All ({count: number}) Connected',
        some: 'Some ({count: number}) Connected',
        none: 'None connected',
        error: 'Status check failed',
      },
      messages: {
        error: 'Failed to get gateways status',
        deleteError: 'Failed to delete gateway',
      },
    },
    noLicenseBox: {
      footer: {
        get: 'Get an enterprise license',
        contact: 'by contacting:',
      },
    },
    locationMfaModeSelect: {
      label: 'MFA Requirement',
      options: {
        disabled: 'Do not enforce MFA',
        internal: 'Internal MFA',
        external: 'External MFA',
      },
    },
  },
  settingsPage: {
    title: 'Settings',
    tabs: {
      smtp: 'SMTP',
      global: 'Global settings',
      ldap: 'LDAP',
      openid: 'OpenID',
      enterprise: 'Enterprise features',
      gatewayNotifications: 'Gateway notifications',
      activityLogStream: 'Activity log streaming',
    },
    messages: {
      editSuccess: 'Settings updated',
      challengeSuccess: 'Challenge message changed',
    },
    enterpriseOnly: {
      title: 'This feature is available only in Defguard Enterprise.',
      currentExpired: 'Your current license has expired.',
      subtitle: 'To learn more, visit our ',
      website: 'website',
    },
    activityLogStreamSettings: {
      messages: {
        destinationCrud: {
          create: '{destination: string} destination added',
          modify: '{destination: string} destination modified',
          delete: '{destination: string} destination removed',
        },
      },
      modals: {
        selectDestination: {
          title: 'Select destination',
        },
        vector: {
          create: 'Add Vector destination',
          modify: 'Edit Vector destination',
        },
        logstash: {
          create: 'Add Logstash destination',
          modify: 'Edit Logstash destination',
        },
        shared: {
          formLabels: {
            name: 'Name',
            url: 'Url',
            username: 'Username',
            password: 'Password',
            cert: 'Certificate',
          },
        },
      },
      title: 'Activity log streaming',
      list: {
        noData: 'No destinations',
        headers: {
          name: 'Name',
          destination: 'Destination',
        },
      },
    },
    ldapSettings: {
      title: 'LDAP Settings',
      sync: {
        header: 'LDAP two-way synchronization',
        info: 'Before enabling synchronization, please read more about it in our [documentation](https://docs.defguard.net/enterprise/all-enteprise-features/ldap-and-active-directory-integration/two-way-ldap-and-active-directory-synchronization).',
        info_enterprise: 'This feature is available only in Defguard Enterprise.',
        helpers: {
          heading:
            'Configure LDAP synchronization settings here. If configured, Defguard will pull user information from LDAP and synchronize it with local users.',
          sync_enabled:
            'If enabled, Defguard will attempt to pull LDAP user data at the specified interval.',
          authority: `Defguard will use the selected server as the authoritative source of
          user data, meaning that if LDAP is selected, Defguard data will be overwritten with the LDAP
          data in case of a desynchronization. If Defguard was selected as the authority, it's data will
          overwrite LDAP data if necessary.
          Make sure to check the documentation to understand the implications of this
          setting.`,
          interval: 'The interval with which the synchronization will be attempted.',
          groups: `Defguard will attempt to synchronize only users belonging to the provided groups. Provide a comma-separated list of groups. If empty, all users will be synchronized.`,
        },
      },
      form: {
        labels: {
          ldap_enable: 'Enable LDAP integration',
          ldap_url: 'URL',
          ldap_bind_username: 'Bind Username',
          ldap_bind_password: 'Bind Password',
          ldap_member_attr: 'Member Attribute',
          ldap_username_attr: 'Username Attribute',
          ldap_user_obj_class: 'User Object Class',
          ldap_user_search_base: 'User Search Base',
          ldap_user_auxiliary_obj_classes: 'Additional User Object Classes',
          ldap_groupname_attr: 'Groupname Attribute',
          ldap_group_search_base: 'Group Search Base',
          ldap_group_member_attr: 'Group Member Attribute',
          ldap_group_obj_class: 'Group Object Class',
          ldap_sync_enabled: 'Enable LDAP two-way synchronization',
          ldap_authoritative_source: 'Consider the following source as the authority',
          ldap_sync_interval: 'Synchronization interval',
          ldap_use_starttls: 'Use StartTLS',
          ldap_tls_verify_cert: 'Verify TLS certificate',
          ldap_uses_ad: 'LDAP server is Active Directory',
          ldap_user_rdn_attr: 'User RDN Attribute',
          ldap_sync_groups: 'Limit synchronization to these groups',
        },
        helpers: {
          ldap_user_obj_class:
            'The object class that will be added to the user object during its creation. This is used to determine if an LDAP object is a user.',
          ldap_user_auxiliary_obj_classes:
            "The additional object classes that will be added to the user object during its creation. They may also influence the added user's attributes (e.g. simpleSecurityObject class will add userPassword attribute).",
          user_settings:
            'Configure LDAP user settings here. These settings determine how Defguard maps and synchronizes LDAP user information with local users.',
          connection_settings:
            'Configure LDAP connection settings here. These settings determine how Defguard connects to your LDAP server. Encrypted connections are also supported (StartTLS, LDAPS).',
          group_settings:
            'Configure LDAP group settings here. These settings determine how Defguard maps and synchronizes LDAP group information with local groups.',
          ldap_group_obj_class:
            'The object class that represents a group in LDAP. This is used to determine if an LDAP object is a group.',
          ldap_user_rdn_attr:
            "If your user's RDN attribute is different than your username attribute, please provide it here, otherwise leave it empty to use the username attribute as the user's RDN.",
        },
        headings: {
          user_settings: 'User settings',
          connection_settings: 'Connection settings',
          group_settings: 'Group settings',
        },
        delete: 'Delete configuration',
      },
      test: {
        title: 'Test LDAP Connection',
        submit: 'Test',
        messages: {
          success: 'LDAP connected successfully',
          error: 'LDAP connection rejected',
        },
      },
    },
    openIdSettings: {
      heading: 'External OpenID settings',
      general: {
        title: 'General settings',
        helper: 'Here you can change general OpenID behavior in your Defguard instance.',
        createAccount: {
          label:
            'Automatically create user account when logging in for the first time through external OpenID.',
          helper:
            'If this option is enabled, Defguard automatically creates new accounts for users who log in for the first time using an external OpenID provider. Otherwise, the user account must first be created by an administrator.',
        },
        usernameHandling: {
          label: 'Username handling',
          helper:
            'Configure the method for handling invalid characters in usernames provided by your identity provider.',
          options: {
            remove: 'Remove forbidden characters',
            replace: 'Replace forbidden characters',
            prune_email: 'Prune email domain',
          },
        },
      },
      form: {
        title: 'Client settings',
        helper:
          'Here you can configure the OpenID client settings with values provided by your external OpenID provider.',
        custom: 'Custom',
        none: 'None',
        documentation:
          'Make sure to check our [documentation](https://docs.defguard.net/enterprise/all-enteprise-features/external-openid-providers) for more information and examples.',
        delete: 'Delete provider',
        directory_sync_settings: {
          title: 'Directory synchronization settings',
          helper:
            "Directory synchronization allows you to automatically synchronize users' status and groups from an external provider.",
          notSupported: 'Directory sync is not supported for this provider.',
          connectionTest: {
            success: 'Connection successful',
            error: 'Connection failed with error:',
          },
        },
        selects: {
          synchronize: {
            all: 'All',
            users: 'Users',
            groups: 'Groups',
          },
          behavior: {
            keep: 'Keep',
            disable: 'Disable',
            delete: 'Delete',
          },
        },
        labels: {
          provider: {
            label: 'Provider',
            helper:
              'Select your OpenID provider. You can use custom provider and fill in the base URL by yourself.',
          },
          client_id: {
            label: 'Client ID',
            helper: 'Client ID provided by your OpenID provider.',
          },
          client_secret: {
            label: 'Client Secret',
            helper: 'Client Secret provided by your OpenID provider.',
          },
          base_url: {
            label: 'Base URL',
            helper:
              'Base URL of your OpenID provider, e.g. https://accounts.google.com. Make sure to check our documentation for more information and examples.',
          },
          display_name: {
            label: 'Display Name',
            helper:
              "Name of the OpenID provider to display on the login's page button. If not provided, the button will display generic 'Login with OIDC' text.",
          },
          enable_directory_sync: {
            label: 'Enable directory synchronization',
          },
          sync_target: {
            label: 'Synchronize',
            helper:
              "What to synchronize from the external provider. You can choose between synchronizing both users' state and group memberships, or narrow it down to just one of these.",
          },
          sync_interval: {
            label: 'Synchronization interval',
            helper: 'Interval in seconds between directory synchronizations.',
          },
          user_behavior: {
            label: 'User behavior',
            helper:
              'Choose how to handle users that are not present in the external provider anymore. You can select between keeping, disabling, or deleting them.',
          },
          admin_behavior: {
            label: 'Admin behavior',
            helper:
              'Choose how to handle Defguard admins that are not present in the external provider anymore. You can select between keeping them, disabling them or completely deleting them.',
          },
          admin_email: {
            label: 'Admin email',
            helper:
              'Email address of the account on which behalf the synchronization checks will be performed, e.g. the person who setup the Google service account. See our documentation for more details.',
          },
          service_account_used: {
            label: 'Service account in use',
            helper:
              'The service account currently being used for synchronization. You can change it by uploading a new service account key file.',
          },
          service_account_key_file: {
            label: 'Service Account Key file',
            helper:
              "Upload a new service account key file to set the service account used for synchronization. NOTE: The uploaded file won't be visible after saving the settings and reloading the page as it's contents are sensitive and are never sent back to the dashboard.",
            uploaded: 'File uploaded',
            uploadPrompt: 'Upload a service account key file',
          },
          okta_client_id: {
            label: 'Directory Sync Client ID',
            helper: 'Client ID for the Okta directory sync application.',
          },
          okta_client_key: {
            label: 'Directory Sync Client Private Key',
            helper:
              "Client private key for the Okta directory sync application in the JWK format. It won't be shown again here.",
          },
          group_match: {
            label: 'Sync only matching groups',
            helper:
              'Provide a comma separated list of group names that should be synchronized. If left empty, all groups from the provider will be synchronized.',
          },
        },
      },
    },
    modulesVisibility: {
      header: 'Modules Visibility',
      helper: `<p>
            Hide unused modules.
          </p>
          <a href="{documentationLink:string}" target="_blank">
            Read more in documentation.
          </a>`,
      fields: {
        wireguard_enabled: {
          label: 'WireGuard VPN',
        },
        webhooks_enabled: {
          label: 'Webhooks',
        },
        worker_enabled: {
          label: 'Yubikey provisioning',
        },
        openid_enabled: {
          label: 'OpenID Connect',
        },
      },
    },
    defaultNetworkSelect: {
      header: 'Default location view',
      helper: `<p>Here you can change your default location view.</p>
          <a href="{documentationLink:string}" target="_blank">
            Read more in documentation.
          </a>`,
      filterLabels: {
        grid: 'Grid view',
        list: 'List view',
      },
    },
    instanceBranding: {
      header: 'Instance Branding',
      form: {
        title: 'Name & Logo:',
        fields: {
          instanceName: {
            label: 'Instance name',
            placeholder: 'Defguard',
          },
          mainLogoUrl: {
            label: 'Login logo url',
            helper: 'Maximum picture size is 250x100  px',
            placeholder: 'Default image',
          },
          navLogoUrl: {
            label: 'Menu & navigation small logo',
            helper: 'Maximum picture size is 100x100 px',
            placeholder: 'Default image',
          },
        },
        controls: {
          restoreDefault: 'Restore default',
          submit: 'Save changes',
        },
      },
      helper: `
			      <p>
            Here you can add url of your logo and name for your defguard
            instance it will be displayed instead of defguard.
          </p>
          <a href="{documentationLink:string}" target="_blank">
            Read more in documentation.
          </a>
			`,
    },
    license: {
      header: 'Enterprise',
      helpers: {
        enterpriseHeader: {
          text: 'Here you can manage your Defguard Enterprise version license.',
          link: 'To learn more about Defguard Enterprise, visit our webiste.',
        },
        licenseKey: {
          text: 'Enter your Defguard Enterprise license key below. You should receive it via email after purchasing the license.',
          link: 'You can purchase the license here.',
        },
      },
      form: {
        title: 'License',
        fields: {
          key: {
            label: 'License key',
            placeholder: 'Your Defguard license key',
          },
        },
      },
      licenseInfo: {
        title: 'License information',
        status: {
          noLicense: 'No valid license',
          expired: 'Expired',
          limitsExceeded: 'Limits Exceeded',
          active: 'Active',
        },
        licenseNotRequired:
          "<p>You have access to this enterprise feature, as you haven't exceeded any of the usage limits yet. Check the <a href='https://docs.defguard.net/enterprise/license'>documentation</a> for more information.</p>",
        types: {
          subscription: {
            label: 'Subscription',
            helper: 'A license that automatically renews at regular intervals',
          },
          offline: {
            label: 'Offline',
            helper:
              'The license is valid until the expiry date and does not automatically renew',
          },
        },
        fields: {
          status: {
            label: 'Status',
            active: 'Active',
            expired: 'Expired',
            subscriptionHelper:
              'A subscription license is considered valid for some time after the expiration date to account for possible automatic payment delays.',
          },
          type: {
            label: 'Type',
          },
          validUntil: {
            label: 'Valid until',
          },
        },
      },
    },
    smtp: {
      form: {
        title: 'SMTP configuration',
        sections: {
          server: 'Server settings',
        },
        fields: {
          encryption: {
            label: 'Encryption',
          },
          server: {
            label: 'Server address',
            placeholder: 'Address',
          },
          port: {
            label: 'Server port',
            placeholder: 'Port',
          },
          user: {
            label: 'Server username',
            placeholder: 'Username',
          },
          password: {
            label: 'Server password',
            placeholder: 'Password',
          },
          sender: {
            label: 'Sender email address',
            placeholder: 'Address',
            helper: `
              <p>
                System messages will be sent from this address.
                E.g. no-reply@my-company.com.
              </p>
            `,
          },
        },
        controls: {
          submit: 'Save changes',
        },
      },
      delete: 'Delete configuration',
      testForm: {
        title: 'Send test email',
        subtitle: 'Enter recipent email address',
        fields: {
          to: {
            label: 'Send test email to',
            placeholder: 'Address',
          },
        },
        controls: {
          submit: 'Send',
          resend: 'Resend',
          retry: 'Retry',
          success: 'Test email sent',
          error: 'Error sending email',
        },
        success: {
          message: 'Test email has been sent successully.',
        },
        error: {
          message:
            'There was an error sending the test email. Please check your SMTP configuration and try again.',
          fullError: 'Error: {error: string}',
        },
      },
      helper: `Here you can configure SMTP server used to send system messages to the users.`,
    },
    enrollment: {
      helper:
        'Enrollment is a process by which a new employee will be able to activate their new account, create a password and configure a VPN device.',
      vpnOptionality: {
        header: 'VPN step optionality',
        helper:
          'You can choose whether creating a VPN device is optional or mandatory during enrollment',
      },
      welcomeMessage: {
        header: 'Welcome message',
        helper: `
        <p>In this text input you can use Markdown:</p>
        <ul>
          <li>Headings start with a hash #</li>
          <li>Use asterisks for <i>*italics*</i></li>
          <li>Use two asterisks for <b>**bold**</b></li>
        </ul>
        `,
      },
      welcomeEmail: {
        header: 'Welcome e-mail',
        helper: `
        <p>In this text input you can use Markdown:</p>
        <ul>
          <li>Headings start with a hash #</li>
          <li>Use asterisks for <i>*italics*</i></li>
          <li>Use two asterisks for <b>**bold**</b></li>
        </ul>
        `,
      },
      form: {
        controls: {
          submit: 'Save changes',
        },
        welcomeMessage: {
          helper:
            'This information will be displayed for the user once enrollment is completed. We advise you to insert relevant links and explain next steps briefly.',
          placeholder: 'Please input welcome message',
        },
        welcomeEmail: {
          helper:
            'This information will be sent to the user once enrollment is completed. We advise you to insert relevant links and explain next steps briefly. You can reuse the welcome message here.',
          placeholder: 'Please input welcome email',
        },
        welcomeEmailSubject: {
          label: 'Subject',
        },
        useMessageAsEmail: {
          label: 'Same as welcome message',
        },
      },
    },
    enterprise: {
      header: 'Enterprise Features',
      helper: 'Here you can change enterprise settings.',
      fields: {
        deviceManagement: {
          label: "Disable users' ability to manage their devices",
          helper:
            "When this option is enabled, only users in the Admin group can manage devices in user profile (it's disabled for all other users)",
        },
        disableAllTraffic: {
          label: 'Disable the option to route all traffic through VPN',
          helper:
            'When this option is enabled, users will not be able to route all traffic through the VPN using the defguard client.',
        },
        manualConfig: {
          label: "Disable users' ability to manually configure WireGuard client",
          helper:
            "When this option is enabled, users won't be able to view or download configuration for the manual WireGuard client setup. Only the Defguard desktop client configuration will be available.",
        },
      },
    },
    gatewayNotifications: {
      smtpWarning: 'To enable notifications you must first configure an SMTP server',
      header: 'Notifications',
      sections: {
        gateway: 'Gateway disconnect notifications',
      },
      helper: 'Here you can manage email notifications.',
      form: {
        submit: 'Save changes',
        fields: {
          disconnectNotificationsEnabled: {
            label: 'Enable gateway disconnect notifications',
            help: 'Send email notification to admin users once a gateway is disconnected',
          },
          inactivityThreshold: {
            label: 'Gateway inactivity time [minutes]',
            help: 'Time (in minutes) that a gateway needs to stay disconnected before a notification is sent',
          },
          reconnectNotificationsEnabled: {
            label: 'Enable gateway reconnect notifications',
            help: 'Send email notification to admin users once a gateway is reconnected',
          },
        },
      },
    },
  },
  openidOverview: {
    pageTitle: 'OpenID Apps',
    search: {
      placeholder: 'Find apps',
    },
    filterLabels: {
      all: 'All apps',
      enabled: 'Enabled',
      disabled: 'Disabled',
    },
    clientCount: 'All apps',
    addNewApp: 'Add new',
    list: {
      headers: {
        name: 'Name',
        status: 'Status',
        actions: 'Actions',
      },
      editButton: {
        edit: 'Edit app',
        delete: 'Delete app',
        disable: 'Disable',
        enable: 'Enable',
        copy: 'Copy client ID',
      },
      status: {
        enabled: 'Enabled',
        disabled: 'Disabled',
      },
    },
    messages: {
      copySuccess: 'Client ID copied.',
      noLicenseMessage: "You don't have a license for this feature.",
      noClientsFound: 'No results found.',
    },
    deleteApp: {
      title: 'Delete app',
      message: 'Do you want to delete {appName: string} app ?',
      submit: 'Delete app',
      messages: {
        success: 'App deleted.',
      },
    },
    enableApp: {
      messages: {
        success: 'App enabled.',
      },
    },
    disableApp: {
      messages: {
        success: 'App disabled.',
      },
    },
    modals: {
      openidClientModal: {
        title: {
          addApp: 'Add Application',
          editApp: 'Edit {appName: string} app',
        },
        scopes: 'Scopes:',
        messages: {
          clientIdCopy: 'Client ID copied.',
          clientSecretCopy: 'Client secret copied.',
        },
        form: {
          messages: {
            successAdd: 'App created.',
            successModify: 'App modified.',
          },
          error: {
            urlRequired: 'URL is required.',
            validUrl: 'Must be a valid URL.',
            scopeValidation: 'Must have at least one scope.',
          },
          fields: {
            name: {
              label: 'App name',
            },
            redirectUri: {
              label: 'Redirect URL {count: number}',
              placeholder: 'https://example.com/redirect',
            },
            openid: {
              label: 'OpenID',
            },
            profile: {
              label: 'Profile',
            },
            email: {
              label: 'Email',
            },
            phone: {
              label: 'Phone',
            },
            groups: {
              label: 'Groups',
            },
          },
          controls: {
            addUrl: 'Add URL',
          },
        },
        clientId: 'Client ID',
        clientSecret: 'Client secret',
      },
    },
  },
  webhooksOverview: {
    pageTitle: 'Webhooks',
    search: {
      placeholder: 'Find webhooks by url',
    },
    filterLabels: {
      all: 'All webhooks',
      enabled: 'Enabled',
      disabled: 'Disabled',
    },
    webhooksCount: 'All webhooks',
    addNewWebhook: 'Add new',
    noWebhooksFound: 'No webhooks found.',
    list: {
      headers: {
        name: 'Name',
        description: 'Description',
        status: 'Status',
        actions: 'Actions',
      },
      editButton: {
        edit: 'Edit',
        delete: 'Delete webhook',
        disable: 'Disable',
        enable: 'Enable',
      },
      status: {
        enabled: 'Enabled',
        disabled: 'Disabled',
      },
    },
  },
  provisionersOverview: {
    pageTitle: 'Provisioners',
    search: {
      placeholder: 'Find provisioners',
    },
    filterLabels: {
      all: 'All',
      available: 'Available',
      unavailable: 'Unavailable',
    },
    provisionersCount: 'All provisioners',
    noProvisionersFound: 'No provisioners found.',
    noLicenseMessage: "You don't have a license for this feature.",
    provisioningStation: {
      header: 'YubiKey provisioning station',
      content: `In order to be able to provision your YubiKeys, first you need to set up
        physical machine with USB slot. Run provided command on your chosen
        machine to register it and start provisioning your keys.`,
      dockerCard: {
        title: 'Provisioning station docker setup command',
      },
      tokenCard: {
        title: 'Access token',
      },
    },
    list: {
      headers: {
        name: 'Name',
        ip: 'IP address',
        status: 'Status',
        actions: 'Actions',
      },
      editButton: {
        delete: 'Delete provisioner',
      },
      status: {
        available: 'Available',
        unavailable: 'Unavailable',
      },
    },
    messages: {
      copy: {
        token: 'Token copied',
        command: 'Command copied',
      },
    },
  },
  openidAllow: {
    header: '{name: string} would like to:',
    scopes: {
      openid: 'Use your profile data for future logins.',
      profile: 'Know basic information from your profile like name, profile picture etc.',
      email: 'Know your email address.',
      phone: 'Know your phone number.',
      groups: 'Know your groups membership.',
    },
    controls: {
      accept: 'Accept',
      cancel: 'Cancel',
    },
  },
  networkOverview: {
    networkSelection: {
      all: 'All locations summary',
      placeholder: 'Select location',
    },
    timeRangeSelectionLabel: '{value: number}h period',
    pageTitle: 'Location overview',
    controls: {
      editNetworks: 'Edit Locations settings',
      selectNetwork: {
        placeholder: 'Loading locations',
      },
    },
    filterLabels: {
      grid: 'Grid view',
      list: 'List view',
    },
    gatewayStatus: {
      all: 'All ({count: number}) Connected',
      some: 'Some ({count: number}) Connected',
      none: 'None connected',
    },
    stats: {
      currentlyActiveUsers: 'Currently active users',
      currentlyActiveNetworkDevices: 'Currently active network devices',
      totalUserDevices: 'Total user devices: {count: number}',
      activeNetworkDevices: 'Active network devices in {hour: number}h',
      activeUsersFilter: 'Active users in {hour: number}h',
      activeDevicesFilter: 'Active devices in {hour: number}h',
      activityIn: 'Activity in {hour: number}H',
      networkUsage: 'Network usage',
      peak: 'Peak',
      in: 'In:',
      out: 'Out:',
      gatewayDisconnected: 'Gateway disconnected',
    },
    cardsLabels: {
      users: 'Connected Users',
      devices: 'Connected Network Devices',
    },
  },
  connectedUsersOverview: {
    pageTitle: 'Connected users',
    noUsersMessage: 'Currently there are no connected users',
    userList: {
      username: 'Username',
      device: 'Device',
      connected: 'Connected',
      deviceLocation: 'Device location',
      networkUsage: 'Network usage',
    },
  },
  networkPage: {
    pageTitle: 'Edit Location',
    addNetwork: '+ Add new location',
    controls: {
      networkSelect: {
        label: 'Location choice',
      },
    },
  },
  activityOverview: {
    header: 'Activity stream',
    noData: 'Currently there is no activity detected',
  },
  networkConfiguration: {
    messages: {
      delete: {
        success: 'Network deleted',
        error: 'Failed to delete network',
      },
    },
    header: 'Location configuration',
    importHeader: 'Location import',
    form: {
      helpers: {
        address:
          'Based on this address VPN network address will be defined, eg. 10.10.10.1/24 (and VPN network will be: 10.10.10.0/24). You can optionally specify multiple addresses separated by a comma. The first address is the primary address, and this one will be used for IP address assignment for devices. The other IP addresses are auxiliary and are not managed by Defguard.',
        gateway: 'Gateway public address, used by VPN users to connect',
        dns: 'Specify the DNS resolvers to query when the wireguard interface is up.',
        allowedIps:
          'List of addresses/masks that should be routed through the VPN network.',
        allowedGroups:
          'By default, all users will be allowed to connect to this location. If you want to restrict access to this location to a specific group, please select it below.',
        aclFeatureDisabled:
          "ACL functionality is an enterprise feature and you've exceeded the user, device or network limits to use it. In order to use this feature, purchase an enterprise license or upgrade your existing one.",
      },
      messages: {
        networkModified: 'Location modified.',
        networkCreated: 'Location created',
      },
      fields: {
        name: {
          label: 'Location name',
        },
        address: {
          label: 'Gateway VPN IP address and netmask',
        },
        endpoint: {
          label: 'Gateway address',
        },
        allowedIps: {
          label: 'Allowed Ips',
        },
        port: {
          label: 'Gateway port',
        },
        dns: {
          label: 'DNS',
        },
        allowedGroups: {
          label: 'Allowed groups',
          placeholder: 'All groups',
        },
        keepalive_interval: {
          label: 'Keepalive interval [seconds]',
        },
        peer_disconnect_threshold: {
          label: 'Peer disconnect threshold [seconds]',
        },
        acl_enabled: {
          label: 'Enable ACL for this location',
        },
        acl_default_allow: {
          label: 'Default ACL policy',
        },
      },
      controls: {
        submit: 'Save changes',
        cancel: 'Back to Overview',
        delete: 'Remove location',
      },
    },
  },
  gatewaySetup: {
    header: {
      main: 'Gateway server setup',
      dockerBasedGatewaySetup: `Docker Based Gateway Setup`,
      fromPackage: `From Package`,
      oneLineInstall: `One Line Install`,
    },
    card: {
      title: 'Docker based gateway setup',
      authToken: `Authentication Token`,
    },
    button: {
      availablePackages: `Available Packages`,
    },
    controls: {
      status: 'Check connection status',
    },
    messages: {
      runCommand: `Defguard requires to deploy a gateway node to control wireguard VPN on the vpn server.
            More details can be found in the [documentation]({setupGatewayDocs:string}).
            There are several ways to deploy the gateway server,
            below is a Docker based example, for other examples please visit [documentation]({setupGatewayDocs:string}).`,
      createNetwork: `Please create the network before running the gateway process.`,
      noConnection: `No connection established, please run provided command.`,
      connected: `Gateway connected.`,
      statusError: 'Failed to get gateway status',
      oneLineInstall: `If you are doing one line install: https://docs.defguard.net/admin-and-features/setting-up-your-instance/one-line-install
          you don't need to do anything.`,
      fromPackage: `Install the package available at https://github.com/DefGuard/gateway/releases/latest and configure \`/etc/defguard/gateway.toml\`
          according to the [documentation]({setupGatewayDocs:string}).`,
      authToken: `Token below is required to authenticate and configure the gateway node. Ensure you keep this token secure and follow the deployment instructions
          provided in the [documentation]({setupGatewayDocs:string}) to successfully set up the gateway server.
          For more details and exact steps, please refer to the [documentation]({setupGatewayDocs:string}).`,
      dockerBasedGatewaySetup: `Below is a Docker based example. For more details and exact steps, please refer to the [documentation]({setupGatewayDocs:string}).`,
    },
  },
  loginPage: {
    pageTitle: 'Enter your credentials',
    oidcLogin: 'Sign in with',
    callback: {
      return: 'Go back to login',
      error: 'An error occurred during external OpenID login',
    },
    mfa: {
      title: 'Two-factor authentication',
      controls: {
        useAuthenticator: 'Use Authenticator app instead',
        useWebauthn: 'Use security key instead',
        useRecoveryCode: 'Use recovery code instead',
        useEmail: 'Use E-mail instead',
      },
      email: {
        header: 'Use code we sent to your e-mail to proceed.',
        form: {
          labels: {
            code: 'Code',
          },
          controls: {
            resendCode: 'Resend Code',
          },
        },
      },
      totp: {
        header: 'Use code from your authentication app and click button to proceed.',
        form: {
          fields: {
            code: {
              placeholder: 'Enter Authenticator code',
            },
          },
          controls: {
            submit: 'Use authenticator code',
          },
        },
      },
      recoveryCode: {
        header: 'Enter one of active recovery codes and click button to log in.',
        form: {
          fields: {
            code: {
              placeholder: 'Recovery code',
            },
          },
          controls: {
            submit: 'Use recovery code',
          },
        },
      },
      webauthn: {
        header: 'When you are ready to authenticate, press the button below.',
        controls: {
          submit: 'Use security key',
        },
        messages: {
          error: 'Failed to read key. Please try again.',
        },
      },
    },
  },
  wizard: {
    completed: 'Location setup completed',
    configuration: {
      successMessage: 'Location created',
    },
    welcome: {
      header: 'Welcome to location wizard!',
      sub: 'Before you start using VPN you need to setup your first location. When in doubt click on <React> icon.',
      button: 'Setup location',
    },
    navigation: {
      top: 'Location setup',
      titles: {
        welcome: 'Location setup',
        choseNetworkSetup: 'Chose Location setup',
        importConfig: 'Import existing location',
        manualConfig: 'Configure location',
        mapDevices: 'Map imported devices',
      },
      buttons: {
        next: 'Next',
        back: 'Back',
      },
    },
    deviceMap: {
      messages: {
        crateSuccess: 'Devices added',
        errorsInForm: 'Please fill marked fields.',
      },
      list: {
        headers: {
          deviceName: 'Device Name',
          deviceIP: 'IP',
          user: 'User',
        },
      },
    },
    wizardType: {
      manual: {
        title: 'Manual Configuration',
        description: 'Manual location configuration',
      },
      import: {
        title: 'Import From File',
        description: 'Import from WireGuard config file',
      },
      createNetwork: 'Create location',
    },
    common: {
      select: 'Select',
    },
    locations: {
      form: {
        name: 'Name',
        ip: 'IP address',
        user: 'User',
        fileName: 'File',
        selectFile: 'Select file',
        messages: { devicesCreated: 'Devices created' },
        validation: { invalidAddress: 'Invalid address' },
      },
    },
  },
  layout: {
    select: {
      addNewOptionDefault: 'Add new +',
    },
  },
  redirectPage: {
    title: 'You have been logged in',
    subtitle: 'You will be redirected in a moment...',
  },
  enrollmentPage: {
    title: 'Enrollment',
    controls: {
      default: 'Restore default',
      save: 'Save changes',
    },
    messages: {
      edit: {
        success: 'Settings changed',
        error: 'Save failed',
      },
    },
    messageBox:
      'Enrollment is a process by which a new employee will be able to activate their new account, create a password and configure a VPN device. You can customize it here.',
    settings: {
      welcomeMessage: {
        title: 'Welcome message',
        messageBox:
          'This information will be displayed for user in service once enrollment is completed. We advise to insert links and explain next steps briefly. You can use same message as in the e-mail.',
      },
      vpnOptionality: {
        title: 'VPN set optionallity',
        select: {
          options: {
            optional: 'Optional',
            mandatory: 'Mandatory',
          },
        },
      },
      welcomeEmail: {
        title: 'Welcome e-mail',
        subject: {
          label: 'E-mail subject',
        },
        messageBox:
          'This information will be sent to user once enrollment is completed. We advise to insert links and explain next steps briefly.',
        controls: {
          duplicateWelcome: 'Same as welcome message',
        },
      },
    },
  },
  supportPage: {
    title: 'Support',
    modals: {
      confirmDataSend: {
        title: 'Send Support Data',
        subTitle:
          'Please confirm that you actually want to send support debug information. None of your private information will be sent (wireguard keys, email addresses, etc. will not be sent).',
        submit: 'Send support data',
      },
    },
    debugDataCard: {
      title: 'Support data',
      body: `
If you need assistance or you were asked to generate support data by our team (for example on our Matrix support channel: **#defguard-support:teonite.com**), you have two options:
* Either you can configure SMTP settings and click "Send support data"
* Or click "Download support data" and create a bug report in our GitHub attaching this file.
`,
      downloadSupportData: 'Download support data',
      downloadLogs: 'Download logs',
      sendMail: 'Send support data',
      mailSent: 'Email sent',
      mailError: 'Error sending email',
    },
    supportCard: {
      title: 'Support',
      body: `
Before contacting or submitting any issues to GitHub please get familiar with Defguard documentation available at [docs.defguard.net](https://docs.defguard.net/)

To submit:
* Bugs - please go to [GitHub](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=bug&template=bug_report.md&title=)
* Feature request - please go to [GitHub](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=feature&template=feature_request.md&title=)

Any other requests you can reach us at: support@defguard.net
`,
    },
  },
  devicesPage: {
    title: 'Network Devices',
    search: {
      placeholder: 'Find',
    },
    bar: {
      itemsCount: 'All devices',
      filters: {},
      actions: {
        addNewDevice: 'Add new',
      },
    },
    list: {
      columns: {
        labels: {
          name: 'Device Name',
          location: 'Location',
          assignedIps: 'IP Addresses',
          description: 'Description',
          addedBy: 'Added By',
          addedAt: 'Add Date',
          edit: 'Edit',
        },
      },
      edit: {
        actionLabels: {
          config: 'View config',
          generateToken: 'Generate auth token',
        },
      },
    },
  },
  acl: {
    messageBoxes: {
      aclAliasKind: {
        component: {
          name: 'Component',
          description: 'combined with manually configured destination fields in ACL',
        },
        destination: {
          name: 'Destination',
          description: 'translated into a separate set of firewall rules',
        },
      },
      networkSelectionIndicatorsHelper: {
        //md
        denied: `
          Location access **denied** by default – network traffic not explicitly defined by the rules will be blocked.
          `,
        //md
        allowed: `
          Location access **allowed** by default – network traffic not explicitly defined by the rules will be passed.
          `,
        //md
        unmanaged: `
          Location access unmanaged (ACL disabled)
          `,
      },
    },
    sharedTitle: 'Access Control List',
    fieldsSelectionLabels: {
      ports: 'All ports',
      protocols: 'All protocols',
    },
    ruleStatus: {
      new: 'New',
      applied: 'Applied',
      modified: 'Pending Change',
      deleted: 'Pending Deletion',
      enable: 'Enable',
      enabled: 'Enabled',
      disable: 'Disable',
      disabled: 'Disabled',
      expired: 'Expired',
    },
    listPage: {
      tabs: {
        rules: 'Rules',
        aliases: 'Aliases',
      },
      message: {
        changeDiscarded: 'Change discarded',
        changeAdded: 'Pending change added',
        changeFail: 'Failed to make change',
        applyChanges: 'Pending changes applied',
        applyFail: 'Failed to apply changes',
      },
      rules: {
        modals: {
          applyConfirm: {
            title: 'Deploy pending changes',
            subtitle: '{count: number} changes will be deployed',
            submit: 'Deploy changes',
          },
          filterGroupsModal: {
            groupHeaders: {
              alias: 'Aliases',
              location: 'Locations',
              groups: 'Groups',
              status: 'Status',
            },
            submit: 'Save Filter',
          },
        },
        listControls: {
          searchPlaceholder: 'Find name',
          addNew: 'Add new',
          filter: {
            nothingApplied: 'Filter',
            applied: 'Filters ({count: number})',
          },
          apply: {
            noChanges: 'Deploy pending changes',
            all: 'Deploy pending changes ({count: number})',
            selective: 'Deploy selected changes ({count: number})',
          },
        },
        list: {
          pendingList: {
            title: 'Pending Changes',
            noData: 'No pending changes',
            noDataSearch: 'No pending changes found',
          },
          deployedList: {
            title: 'Deployed Rules',
            noData: 'No deployed rules',
            noDataSearch: 'No deployed rules found',
          },
          headers: {
            name: 'Rule name',
            id: 'ID',
            destination: 'Destination',
            allowed: 'Allowed',
            denied: 'Denied',
            locations: 'Locations',
            status: 'Status',
            edit: 'Edit',
          },
          tags: {
            all: 'All',
            allDenied: 'All denied',
            allAllowed: 'All allowed',
          },
          editMenu: {
            discard: 'Discard Changes',
            delete: 'Mark for Deletion',
          },
        },
      },
      aliases: {
        message: {
          rulesApply: 'Pending changes applied',
          rulesApplyFail: 'Failed to apply changes',
          aliasDeleted: 'Alias deleted',
          aliasDeleteFail: 'Alias deletion failed',
        },
        modals: {
          applyConfirm: {
            title: 'Confirm Alias Deployment',
            message: `The updated aliases will modify the following rule(s) currently deployed on the gateway.\nPlease ensure these changes are intended before proceeding.`,
            listLabel: 'Affected Rules',
            submit: 'Deploy Changes',
          },
          deleteBlock: {
            title: 'Deletion blocked',
            //md
            content: `
This alias is currently in use by the following rule(s) and cannot be deleted. To proceed with deletion, you must first remove it from these rules({rulesCount: number}):
`,
          },
          filterGroupsModal: {
            groupLabels: {
              rules: 'Rules',
              status: 'Status',
            },
          },
          create: {
            labels: {
              name: 'Alias name',
              kind: 'Alias kind',
              ip: 'IPv4/6 CIDR range address',
              ports: 'Ports or Port Ranges',
              protocols: 'Protocols',
            },
            placeholders: {
              protocols: 'All Protocols',
              ports: 'All Ports',
              ip: 'All IP addresses',
            },
            kindOptions: {
              destination: 'Destination',
              component: 'Component',
            },
            controls: {
              cancel: 'Cancel',
              edit: 'Edit Alias',
              create: 'Create Alias',
            },
            messages: {
              modified: 'Alias modified',
              created: 'Alias created',
            },
          },
        },
        listControls: {
          searchPlaceholder: 'Find name',
          addNew: 'Add new',
          filter: {
            nothingApplied: 'Filter',
            applied: 'Filters ({count: number})',
          },
          apply: {
            noChanges: 'Deploy pending changes',
            all: 'Deploy pending changes ({count: number})',
            selective: 'Deploy selected changes ({count: number})',
          },
        },
        list: {
          pendingList: {
            title: 'Pending Changes',
            noData: 'No pending changes',
            noDataSearch: 'No pending changes found',
          },
          deployedList: {
            title: 'Deployed Aliases',
            noData: 'No deployed aliases',
            noDataSearch: 'No deployed aliases found',
          },
          headers: {
            id: 'ID',
            name: 'Alias name',
            kind: 'Alias kind',
            ip: 'IPv4/6 CIDR range address',
            ports: 'Ports',
            protocols: 'Protocols',
            status: 'Status',
            edit: 'Edit',
            rules: 'Rules',
          },
          status: {
            applied: 'Applied',
            changed: 'Modified',
          },
          tags: {
            allDenied: 'All denied',
            allAllowed: 'All allowed',
          },
          editMenu: {
            discardChanges: 'Discard changes',
            delete: 'Delete alias',
          },
        },
      },
    },
    createPage: {
      formError: {
        allowDenyConflict: 'Conflicting members',
        allowNotConfigured: 'Must configure some allowed users, groups or devices',
      },
      infoBox: {
        // md
        allowInstructions: `
        Specify one or more fields (Users, Groups or Devices) to define this rule. The rule will consider all inputs provided for matching conditions. Leave any fields blank if not needed.`,
        // md
        destinationInstructions: `
        Specify one or more fields (IP Addresses or Ports) to define this rule. The rule will consider all inputs provided for matching conditions. Leave any fields blank if not needed.`,
      },
      message: {
        create: 'Rule created and added to pending changes.',
        createFail: 'Rule creation failed',
      },
      headers: {
        rule: 'Rule',
        createRule: 'Create Rule',
        allowed: 'Allowed Users/Groups/Devices',
        denied: 'Denied Users/Groups/Devices',
        destination: 'Destination',
      },
      labels: {
        name: 'Rule name',
        priority: 'Priority',
        status: 'Status',
        locations: 'Locations',
        allowAllUsers: 'Allow all users',
        allowAllNetworks: 'Include all locations',
        allowAllNetworkDevices: 'Allow all network devices',
        denyAllUsers: 'Deny all users',
        denyAllNetworkDevices: 'Deny all network devices',
        users: 'Users',
        groups: 'Groups',
        devices: 'Network devices',
        protocols: 'Protocols',
        manualIp: 'IPv4/6 CIDR range or address',
        ports: 'Ports',
        aliases: 'Aliases',
        expires: 'Expiration Date',
        manualInput: 'Manual Input',
      },
      placeholders: {
        allProtocols: 'All protocols',
        allIps: 'All IP addresses',
      },
    },
  },
  activity: {
    title: 'Activity log',
    modals: {
      timeRange: {
        title: 'Activity time',
      },
    },
    list: {
      allLabel: 'All activity',
      headers: {
        date: 'Date',
        user: 'User',
        ip: 'IP',
        location: 'Location',
        event: 'Event',
        module: 'Module',
        device: 'Device',
        description: 'Description',
      },
      noData: {
        data: 'No activities present',
        search: 'No activities found',
      },
    },
  },
  enums: {
    activityLogEventType: {
      user_login: 'User login',
      user_login_failed: 'User login failed',
      user_mfa_login: 'User MFA login',
      user_mfa_login_failed: 'User MFA login failed',
      recovery_code_used: 'Recovery code used',
      user_logout: 'User logout',
      user_added: 'User added',
      user_removed: 'User removed',
      user_modified: 'User modified',
      user_groups_modified: 'User groups modified',
      mfa_enabled: 'MFA enabled',
      mfa_disabled: 'MFA disabled',
      user_mfa_disabled: 'User MFA disabled',
      mfa_totp_enabled: 'MFA TOTP enabled',
      mfa_totp_disabled: 'MFA TOTP disabled',
      mfa_email_enabled: 'MFA email enabled',
      mfa_email_disabled: 'MFA email disabled',
      mfa_security_key_added: 'MFA security key added',
      mfa_security_key_removed: 'MFA security key removed',
      device_added: 'Device added',
      device_removed: 'Device removed',
      device_modified: 'Device modified',
      network_device_added: 'Network device added',
      network_device_removed: 'Network device removed',
      network_device_modified: 'Network device modified',
      activity_log_stream_created: 'Activity log stream created',
      activity_log_stream_modified: 'Activity log stream modified',
      activity_log_stream_removed: 'Activity log stream removed',
      vpn_client_connected: 'VPN client connected',
      vpn_client_disconnected: 'VPN client disconnected',
      vpn_client_connected_mfa: 'VPN client connected to MFA location',
      vpn_client_disconnected_mfa: 'VPN client disconnected from MFA location',
      vpn_client_mfa_failed: 'VPN client failed MFA authentication',
      enrollment_token_added: 'Enrollment token added',
      enrollment_started: 'Enrollment started',
      enrollment_device_added: 'Device added',
      enrollment_completed: 'Enrollment completed',
      password_reset_requested: 'Password reset requested',
      password_reset_started: 'Password reset started',
      password_reset_completed: 'Password reset completed',
      vpn_location_added: 'VPN location added',
      vpn_location_removed: 'VPN location removed',
      vpn_location_modified: 'VPN location modified',
      api_token_added: 'API token added',
      api_token_removed: 'API token removed',
      api_token_renamed: 'API token renamed',
      open_id_app_added: 'OpenID app added',
      open_id_app_removed: 'OpenID app removed',
      open_id_app_modified: 'OpenID app modified',
      open_id_app_state_changed: 'OpenID app state changed',
      open_id_provider_removed: 'OpenID provider removed',
      open_id_provider_modified: 'OpenID provider modified',
      settings_updated: 'Settings updated',
      settings_updated_partial: 'Settings partially updated',
      settings_default_branding_restored: 'Default branding restored',
      groups_bulk_assigned: 'Groups bulk assigned',
      group_added: 'Group added',
      group_modified: 'Group modified',
      group_removed: 'Group removed',
      group_member_added: 'Group member added',
      group_member_removed: 'Group member removed',
      group_members_modified: 'Group members modified',
      web_hook_added: 'Webhook added',
      web_hook_modified: 'Webhook modified',
      web_hook_removed: 'Webhook removed',
      web_hook_state_changed: 'Webhook state changed',
      authentication_key_added: 'Authentication key added',
      authentication_key_removed: 'Authentication key removed',
      authentication_key_renamed: 'Authentication key renamed',
      password_changed: 'Password changed',
      password_changed_by_admin: 'Password changed by admin',
      password_reset: 'Password reset',
      client_configuration_token_added: 'Client configuration token added',
      user_snat_binding_added: 'User SNAT binding added',
      user_snat_binding_modified: 'User SNAT binding modified',
      user_snat_binding_removed: 'User SNAT binding removed',
    },
    activityLogModule: {
      defguard: 'Defguard',
      client: 'Client',
      enrollment: 'Enrollment',
      vpn: 'VPN',
    },
  },
};

export default en;

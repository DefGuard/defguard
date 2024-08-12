/* eslint-disable max-len */
import type { Translation } from '../i18n-types';

const pl: Translation = {
  common: {
    controls: {
      back: 'Wróć',
      next: 'Następny',
      close: 'Zamknij',
      cancel: 'Anuluj',
      finish: 'Zakończ',
      select: 'Wybierz',
      submit: 'Wyślij',
      confirm: 'Potwierdź',
      save: 'Zapisz',
      saveChanges: 'Zapisz zmiany',
      RestoreDefault: 'Przywróć domyślne',
      delete: 'Usuń',
      copy: 'Skopiuj',
      rename: 'Zmień nazwę',
      edit: 'Edytuj',
    },
    conditions: {
      and: 'I',
      equal: 'Równy',
      or: 'Albo',
    },
    key: 'Klucz',
    name: 'Nazwa',
  },
  messages: {
    error: 'Wystąpił błąd.',
    success: 'Operacja zakończyła się sukcesem',
    errorVersion: 'Nie udało się uzyskać wersji aplikacji.',
    clipboard: {
      success: 'Skopiowano do schowka',
      error: 'Schowek nie jest dostępny',
    },
    insecureContext: 'Kontekst nie jest bezpieczny',
  },
  modals: {
    addGroup: {
      groupName: 'Nazwa grupy',
      searchPlaceholder: 'Szukaj',
      selectAll: 'Zaznacz wszystkich',
      submit: 'Stwórz grupę',
      title: 'Dodaj grupę',
    },
    editGroup: {
      groupName: 'Nazwa grupy',
      searchPlaceholder: 'Szukaj',
      selectAll: 'Zaznacz wszystkich',
      submit: 'Zmień grupę',
      title: 'Edytuj grupę',
    },
    deleteGroup: {
      title: 'Usuń grupę {name}',
      subTitle: 'Grupa zostanie nieodwołalnie usunięta.',
      locationListHeader:
        'Ta grupa jest obecnie przypisana do następujących lokalizacji:',
      locationListFooter: `Jeżeli to jedyna dozwolona grupa dla danej lokalizacji, stanie się ona <b>dostępna dla wszystkich użytkowników</b>.`,
      submit: 'Usuń grupę',
      cancel: 'Wróć',
    },
    registerEmailMFA: {
      title: 'Skonfiguruj e-mail MFA',
      form: {
        controls: {
          resend: 'Wyślij kod ponownie',
          submit: 'Zweryfikuj kod',
        },
        fields: {
          code: {
            error: 'Podany kod jest nieprawidłowy',
            label: 'Kod',
          },
        },
      },
      infoMessage: `
      <p>
        Aby zakończyć konfigurację, wpisz kod, który został wysłany na adres: <strong>{email}</strong>
      </p>
      `,
      messages: {
        resend: 'Kod wysłany ponownie',
        success: 'Metoda MFA e-mail włączona',
      },
    },
    deviceConfig: {
      title: 'Konfiguracje VPN urządzenia',
    },
    changePasswordSelf: {
      title: 'Zmień hasło',
      messages: {
        success: 'Hasło zostało zmienione',
        error: 'Błąd zmiany hasła',
      },
      form: {
        labels: {
          repeat: 'Powtórz hasło',
          newPassword: 'Nowe hasło',
          oldPassword: 'Obecne hasło',
        },
      },
      controls: {
        cancel: 'Wróć',
        submit: 'Zmień hasło',
      },
    },
    startEnrollment: {
      title: 'Rozpocznij rejestrację',
      desktopTitle: 'Konfiguracja klienta desktop',
      messages: {
        success: 'Rejestracja użytkownika rozpoczęta',
        successDesktop: 'Konfiguracja klienta rozpoczęta',
        errorDesktop: 'Błąd konfiguracji klienta desktop',
        error: 'Błąd rejestracji użytkownika',
      },
      form: {
        email: {
          label: 'E-mail',
        },
        mode: {
          options: {
            email: 'Wyślij token przez e-mail',
            manual: 'Przekaż token ręcznie',
          },
        },
        submit: 'Rozpocznij rejestrację',
        submitDesktop: 'Aktywacja desktop',
        smtpDisabled:
          'Skonfiguruj SMTP, żeby wysłać token przez e-mail. Przejdź do Ustawienia -> SMTP.',
      },
      tokenCard: {
        title: 'Token aktywacji',
      },
      urlCard: {
        title: 'URL instancji Defguard',
      },
    },
    deleteNetwork: {
      cancel: 'Wróć',
      submit: 'Usuń lokalizację',
      subTitle: 'Lokalizacja zostanie nieodwołalnie usunięta.',
      title: 'Usuń lokalizację {name}',
    },
    changeWebhook: {
      messages: {
        success: 'Webhook zmieniony.',
      },
    },
    manageWebAuthNKeys: {
      title: 'Klucze bezpieczeństwa',
      messages: {
        deleted: 'Klucz WebAuthN został usunięty.',
        duplicateKeyError: 'Klucz jest już zarejestrowany',
      },
      infoMessage: `
        <p>
          Klucze bezpieczeństwa mogą być używane jako drugi czynnik uwierzytelniający
          zamiast kodu weryfikacyjnego. Dowiedz się więcej o konfiguracji
          klucza bezpieczeństwa.
        </p>
`,
      form: {
        messages: {
          success: 'Klucz bezpieczeństwa dodany.',
        },
        fields: {
          name: {
            label: 'Nazwa nowego klucza',
          },
        },
        controls: {
          submit: 'Dodaj nowy klucz',
        },
      },
    },
    recoveryCodes: {
      title: 'Kody odzysku',
      submit: 'Zapisałem swoje kody',
      messages: {
        copied: 'Kody skopiowane.',
      },
      infoMessage: `
        <p>
          Traktuj swoje kody odzyskiwania z takim samym poziomem uwagi jak
          jak swoje hasło! Zalecamy zapisywanie ich za pomocą menedżera haseł
          takich jak Lastpass, bitwarden czy Keeper.
        </p>
`,
    },
    registerTOTP: {
      title: 'Authenticator App Setup',
      infoMessage: `
        <p>
          Aby skonfigurować MFA, zeskanuj ten kod QR za pomocą aplikacji uwierzytelniającej, a następnie
          wprowadź kod w polu poniżej:
        </p>
`,
      messages: {
        totpCopied: 'Ścieżka TOTP skopiowana.',
        success: 'TOTP Enabled',
      },
      copyPath: 'Skopiowana ścieżka TOTP',
      form: {
        fields: {
          code: {
            label: 'Kod uwierzytelniający',
            error: 'Kod jest nieprawidłowy',
          },
        },
        controls: {
          submit: 'Weryfikuj kod',
        },
      },
    },
    editDevice: {
      title: 'Edytuj urządzenie',
      messages: {
        success: 'Urządzenie zostało zaktualizowane.',
      },
      form: {
        fields: {
          name: {
            label: 'Nazwa urządzenia',
          },
          publicKey: {
            label: 'Klucz publiczny urządzenia (WireGuard)',
          },
        },
        controls: {
          submit: 'Edytuj urządzenie',
        },
      },
    },
    deleteDevice: {
      title: 'Usuń urządzenie',
      message: 'Czy chcesz usunąć urządzenie {deviceName} ?',
      submit: 'Usuń urządzenie',
      messages: {
        success: 'Urządzenie zostało usunięte.',
      },
    },
    addWallet: {
      title: 'Dodaj portfel',
      infoBox: 'Aby dodać portfel ETH konieczne będzie podpisanie wiadomości.',
      form: {
        fields: {
          name: {
            placeholder: 'Nazwa portfela',
            label: 'Nazwa',
          },
          address: {
            placeholder: 'Adres portfela',
            label: 'Adres',
          },
        },
        controls: {
          submit: 'Dodaj portfel',
        },
      },
    },
    keyDetails: {
      title: 'Szczegóły YubiKey',
      downloadAll: 'Pobierz wszystkie klucze',
    },
    deleteUser: {
      title: 'Usuń użytkownika',
      controls: {
        submit: 'Usuń użytkownika',
      },
      message: 'Czy chcesz trwale usunąć konto {username} ?',
      messages: {
        success: '{username} usunięte.',
      },
    },
    disableUser: {
      title: 'Dezaktywuj użytkownika',
      controls: {
        submit: 'Dezaktywuj użytkownika',
      },
      message: 'Czy chcesz dezaktywować użytkownika {username}?',
      messages: {
        success: 'Użytkownik {username} został dezaktywowany.',
      },
    },
    enableUser: {
      title: 'Aktywuj użytkownika',
      controls: {
        submit: 'Aktywuj użytkownika',
      },
      message: 'Czy chcesz aktywować użytkownika {username}?',
      messages: {
        success: 'Użytkownik {username} został aktywowany.',
      },
    },
    deleteProvisioner: {
      title: 'Usuń provisionera',
      controls: {
        submit: 'Usuń provisionera',
      },
      message: 'Czy chcesz usunąć {id} provisionera?',
      messages: {
        success: '{provisioner} usunięty.',
      },
    },
    changeUserPassword: {
      messages: {
        success: 'Hasło zmienione.',
      },
      title: 'Zmiana hasła użytkownika',
      form: {
        controls: {
          submit: 'Zapisz nowe hasło',
        },
        fields: {
          newPassword: {
            label: 'Nowe hasło',
          },
          confirmPassword: {
            label: 'Powtórz hasło',
          },
        },
      },
    },
    provisionKeys: {
      warning: 'Ta operacja bezpowrotnie usunie dane z aplikacji OpenPGP klucza.',
      title: 'Provisionowanie klucza YubiKey:',
      infoBox: `Wybrany provisioner musi mieć podłączony <b>pusty</b> YubiKey.
                Aby zresetować YubiKey uruchom
                <b>gpg --card-edit</b> przed generowaniem kluczy.`,
      selectionLabel:
        'Wybierz jeden z następujących provisionerów, aby wygenrować klucze na YubiKey:',
      noData: {
        workers: 'Nie znaleziono workerów...',
      },
      controls: {
        submit: 'Wygeneruj klucze dla YubiKey',
      },
      messages: {
        success: 'Klucze zostały przetransferowane na YubiKey',
        errorStatus: 'Wystapił błąd podczas pobierania statusu.',
      },
    },
    addUser: {
      messages: {
        userAdded: 'Stworzono użytkownika',
      },
      title: 'Dodaj nowego użytkownika',
      form: {
        submit: 'Dodaj użytkownika',
        fields: {
          username: {
            placeholder: 'login',
            label: 'Login',
          },
          password: {
            placeholder: 'Hasło',
            label: 'Hasło',
          },
          email: {
            placeholder: 'E-mail użytkownika',
            label: 'E-mail użytkownika',
          },
          firstName: {
            placeholder: 'Imię',
            label: 'Imię',
          },
          lastName: {
            placeholder: 'Nazwisko',
            label: 'Ostatnie imię',
          },
          phone: {
            placeholder: 'Telefon',
            label: 'Telefon',
          },
          enableEnrollment: {
            label: 'Użyj zdalnej rejestracji',
          },
        },
      },
    },
    webhookModal: {
      title: {
        addWebhook: 'Dodaj webhook',
        editWebhook: 'Edytuj webhook',
      },
      messages: {
        clientIdCopy: 'Skopiowano identyfikator klienta',
        clientSecretCopy: 'Sekret klienta skopiowany.',
      },
      form: {
        triggers: 'Zdarzenia wyzwalające:',
        messages: {
          successAdd: 'Webhook utworzony.',
          successModify: 'Webhook zmodyfikowany.',
        },
        error: {
          urlRequired: 'URL jest wymagany.',
          validUrl: 'Musi być poprawnym adresem URL.',
          scopeValidation: 'Musi mieć co najmniej jeden wyzwalacz.',
          tokenRequired: 'Token jest wymagany.',
        },
        fields: {
          description: {
            label: 'Opis',
            placeholder: 'Webhook do tworzenia konta gmail na nowym użytkowniku',
          },
          token: {
            label: 'Secret token',
            placeholder: 'Token autoryzacyjny',
          },
          url: {
            label: 'Webhook URL',
            placeholder: 'https://example.com/webhook',
          },
          userCreated: {
            label: 'Stworzenie nowego użytkownika',
          },
          userDeleted: {
            label: 'Użytkownik usunięty',
          },
          userModified: {
            label: 'Użytkownik zmodyfikowany',
          },
          hwkeyProvision: {
            label: 'Stworzenie kluczy na YubiKey dla użytkownika',
          },
        },
      },
    },
    deleteWebhook: {
      title: 'Usuń webhook',
      message: 'Czy chcesz usunąć {name} webhook ?',
      submit: 'Usuń',
      messages: {
        success: 'Webhook usunięty.',
      },
    },
  },
  addDevicePage: {
    title: 'Dodaj urządzenie',
    messages: {
      deviceAdded: 'Urządzenie dodane',
    },
    helpers: {
      setupOpt: `Możesz dodać urządzenie używając naszego klienta lub samemu skonfigurwać urządzenie.`,
    },
    steps: {
      setupDevice: {
        title: 'Dodaj urządzenie',
        form: {
          errors: {
            name: {
              duplicatedName: 'Nazwa jest już zajęta',
            },
          },
          fields: {
            name: {
              label: 'Nazwa',
            },
            publicKey: {
              label: 'Klucz publiczny',
            },
          },
        },
        options: {
          auto: 'Generuj klucze',
          manual: 'Użyj własnych',
        },
        infoMessage: `<p>W razie problemów możesz odwiedzić <a href="{addDevicesDocs}">dokumentacje</a>.</p>`,
      },
      setupMethod: {
        manual: {
          subTitle:
            'Dla zaawansowanych użytkowników, pobierz konfigurację i skonfiguruj VPN na własnych zasadach.',
          link: 'Pobierz WireGuard',
          title: 'Konfiguracja ręczna',
        },
        remote: {
          title: 'Aktywacja klienta desktop',
          link: 'Pobierz klienta Defguard',
          subTitle: 'Prosta konfiguracja jednym tokenem.',
        },
      },
      configDevice: {
        title: 'Skonfiguruj urządzenie',
        messages: {
          copyConfig: 'Konfiguracja skopiowa',
        },
        qrInfo:
          'Użyj poniższych konfiguracji aby połączyć się z wybranymi lokalizacjami.',
        helpers: {
          warningNoNetworks: 'Nie posiadasz dostępu do żadnej sieci.',
          qrHelper: `<p>Możesz skonfigurować WireGuard na telefonie skanując QR kod używając aplikacji WireGuard.</p>`,
          warningAutoMode: `
<p>Uwaga, Defguard nie przechowuje twojego klucza prywatnego. Gdy opuścisz obecną stronę <strong>nie będziesz mógł</strong> pobrać ponownie konfiguracji z kluczem prywatnym.</p>
`,
          warningManualMode: `<p>
Uwaga, podane tutaj konfiguracje nie posiadają klucza prywatnego. Musisz uzupełnić pobraną konfigurację o swój klucz prywatny.
</p>`,
        },
        qrLabel: 'Konfiguracja WireGuard',
        inputNameLabel: 'Nazwa urządzenia',
      },
      copyToken: {
        title: 'Autoryzacja klienta',
        urlCardTitle: 'Url',
        tokenCardTitle: 'Token',
      },
    },
  },
  userPage: {
    title: {
      view: 'Profil użytkownika',
      edit: 'Edycja profilu użytkownika',
    },
    messages: {
      editSuccess: 'Użytkownik zaktualizowany.',
      failedToFetchUserData: 'Błąd pobierania informacji o użytkowniku.',
      passwordResetEmailSent: 'E-mail zerowania hasła został wysłany.',
    },
    userDetails: {
      header: 'Szczegóły profilu',
      messages: {
        deleteApp: 'Aplikacja i wszystkie tokeny usunięte.',
      },
      fields: {
        username: {
          label: 'Nazwa użytkownika',
        },
        firstName: {
          label: 'Imię',
        },
        lastName: {
          label: 'Nazwisko',
        },
        phone: {
          label: 'Numer telefonu',
        },
        email: {
          label: 'E-mail',
        },
        status: {
          label: 'Status',
          active: 'Aktywny',
          disabled: 'Nieaktywny',
        },
        groups: {
          label: 'Grupy użytkowników',
          noData: 'Brak grup',
        },
        apps: {
          label: 'Autoryzowane aplikacje',
          noData: 'Brak autoryzowanych aplikacji',
        },
      },
    },
    userAuthInfo: {
      header: 'Hasło i uwierzytelnienie',
      password: {
        header: 'Ustawienia hasła',
        changePassword: 'Zmiana hasła',
      },
      recovery: {
        header: 'Opcje odzyskiwania danych',
        codes: {
          label: 'Kody odzyskiwania',
          viewed: 'Obejrzane',
        },
      },
      mfa: {
        header: 'Metody dwuskładnikowe',
        edit: {
          disable: 'Wyłącz MFA',
        },
        messages: {
          mfaDisabled: 'MFA wyłączone.',
          OTPDisabled: 'Hasło jednorazowe wyłączone.',
          changeMFAMethod: 'Metoda MFA zmieniona.',
          EmailMFADisabled: 'Metoda e-mail wyłączona.',
        },
        securityKey: {
          singular: 'klucz bezpieczeństwa',
          plural: 'klucze bezpieczeństwa',
        },
        default: 'domyślny',
        enabled: 'Włączony',
        disabled: 'Wyłączony',
        wallet: {
          singular: 'Portfel',
          plural: 'Portfele',
        },
        labels: {
          totp: 'Hasła jednorazowe oparte na czasie',
          webauth: 'Klucze bezpieczeństwa',
          wallets: 'Portfele',
          email: 'E-mail',
        },
        editMode: {
          enable: 'Włącz',
          disable: 'Wyłącz',
          makeDefault: 'Uczyń domyślnym',
          webauth: {
            manage: 'Zarządzaj kluczami bezpieczeństwa',
          },
        },
      },
    },
    controls: {
      editButton: 'Edytuj profil',
      deleteAccount: 'Usuń konto',
    },
    devices: {
      header: 'Urządzenia użytkownika',
      addDevice: {
        web: 'Dodaj nowe urządzenie',
        desktop: 'Dodaj to urządzenie',
      },
      card: {
        labels: {
          noData: 'Nie połączono',
          connectedThrough: 'Połączone przez',
          publicIP: 'Publiczny adres IP',
          connectionDate: 'Data połączenia',
          lastLocation: 'Ostatnie połączenie z',
          active: 'aktywne',
          assignedIp: 'Przydzielony adres IP',
          lastConnected: 'Ostatnio połączone',
        },
        edit: {
          edit: 'Edycja urządzenia',
          delete: 'Usuń urządzenie',
          showConfigurations: 'Pokaż konfiguracje',
        },
      },
    },
    wallets: {
      messages: {
        addressCopied: 'Adres skopiowany.',
        duplicate: {
          primary: 'Podłączony portfel jest już zarejestrowany',
          sub: 'Proszę połączyć nieużywany portfel.',
        },
      },
      header: 'Portfele użytkowników',
      addWallet: 'Dodaj nowy portfel',
      card: {
        address: 'Adres',
        mfaBadge: 'MFA',
        edit: {
          enableMFA: 'Włącz MFA',
          disableMFA: 'Wyłącz MFA',
          delete: 'Usuń',
          copyAddress: 'Skopuj adres',
        },
        messages: {
          deleteSuccess: 'Portfel usunięty',
          enableMFA: 'MFA w portfelu włączone',
          disableMFA: 'MFA w portfelu wyłączone',
        },
      },
    },
    yubiKey: {
      header: 'YubiKey użytkownika',
      provision: 'Sprovisionuj YubiKey',
      keys: {
        pgp: 'Klucz PGP',
        ssh: 'Klucz SSH',
      },
      noLicense: {
        moduleName: 'Moduł YubiKey',
        line1: 'To jest płatny moduł dla YubiKey',
        line2: 'zarządzanie i provisioning.',
      },
    },
    authenticationKeys: {
      header: 'Klucze autoryzacyjne użytkownika',
      addKey: 'Dodaj nowy klucz',
      keysList: {
        common: {
          copy: 'Skopiuj',
          delete: 'Usuń',
          download: 'Pobierz',
          key: 'Klucz',
          rename: 'Zmień nazwę',
          serialNumber: 'Numer seryjny',
        },
      },
      deleteModal: {
        confirmMessage: 'Klucz {name} zostanie trwale usunięty.',
        title: 'Usuń klucz autoryzacyjny',
      },
      addModal: {
        header: 'Dodaj nowy klucz autoryzacyjny',
        keyType: 'Typ Klucza',
        keyForm: {
          labels: {
            key: 'Klucz',
            title: 'Nazwa',
          },
          placeholders: {
            title: 'Nazwa Klucza',
            key: {
              ssh: 'Rozpoczyna się z ‘ssh-rsa’, ‘ecdsa-sha2-nistp256’, ...',
              gpg: 'Rozpoczyna się z ‘-----BEGIN PGP PUBLIC KEY BLOCK-----‘',
            },
          },
          submit: 'Dodaj klucz {name}',
        },
        messages: {
          keyAdded: 'Klucz dodany.',
          keyExists: 'Klucz już został dodany.',
          unsupportedKeyFormat: 'Format klucza nie jest wspierany.',
          genericError: 'Nie udało się dodać klucza. Proszę spróbować ponownie później.',
        },
        yubikeyForm: {
          selectWorker: {
            info: 'Ta operacja wyzeruje moduł GPG do ustawień fabrycznych po czym ponownie go skonfiguruje. Ta operacja jest nieodwracalna.',
            selectLabel: 'Wybierz jedną stację do konfiguracji klucza.',
            noData: 'Obecnie nie ma dostępnych stacji.',
            available: 'Dostępny',
            unavailable: 'Niedostępny',
          },
          provisioning: {
            inProgress: 'Klucz jest konfigurowany, proszę czekać.',
            error: 'Konfiguracja klucza zakończyła się niepowodzeniem.',
            success: 'Klucz skonfigurowany pomyślnie.',
          },
          submit: 'Skonfiguruj klucz',
        },
      },
    },
  },
  usersOverview: {
    pageTitle: 'Użytkownicy',
    search: {
      placeholder: 'Znajdź użytkowników',
    },
    filterLabels: {
      all: 'Wszyscy użytkownicy',
      admin: 'Tylko administratorzy',
      users: 'Tylko użytkownicy',
    },
    usersCount: 'Wszyscy użytkownicy',
    addNewUser: 'Dodaj użytkownika',
    list: {
      headers: {
        name: 'Nazwa użytkownika',
        username: 'Login',
        phone: 'Telefon',
        actions: 'Akcje',
      },
      editButton: {
        activateDesktop: 'Aktywacja klienta desktop',
        changePassword: 'Zmień hasło',
        edit: 'Edytuj konto',
        delete: 'Usuń konto',
        startEnrollment: 'Rozpocznij rejestrację',
        resetPassword: 'Resetuj hasło',
        addGPG: 'Dodaj klucz GPG',
        addSSH: 'Dodaj klucz SSH',
        addYubikey: 'Dodaj YubiKey',
      },
    },
  },
  navigation: {
    bar: {
      overview: 'Przegląd sieci',
      users: 'Użytkownicy',
      provisioners: 'YubiKey Provisioners',
      webhooks: 'Webhooki',
      openId: 'Aplikacje OpenID',
      myProfile: 'Mój profil',
      settings: 'Ustawienia',
      logOut: 'Wyloguj się',
      enrollment: 'Rejestracja',
      support: 'Wsparcie',
      groups: 'Grupy',
    },
    mobileTitles: {
      wizard: 'Konfiguracja VPN',
      users: 'Użytkownicy',
      settings: 'Ustawienia globalne Defguard',
      user: 'Profil użytkownika',
      provisioners: 'YubiKey Provisioners',
      webhooks: 'Webhooki',
      openId: 'Aplikacje OpenID',
      overview: 'Przegląd lokalizacji',
      networkSettings: 'Edycja lokalizacji',
      enrollment: 'Rejestracja',
      support: 'Wsparcie',
      groups: 'Grupy',
    },
    copyright: 'Copyright \u00A9 2023-2024',
    version: {
      open: 'Wersja aplikacji: {version}',
      closed: 'v {version}',
    },
  },
  form: {
    download: 'Pobierz',
    copy: 'Kopiuj',
    saveChanges: 'Zapisz zmiany',
    submit: 'Zapisz',
    login: 'Zaloguj sie',
    cancel: 'Anuluj',
    close: 'Zamknij',
    placeholders: {
      password: 'Hasło',
      username: 'Nazwa użytkownika',
    },
    error: {
      invalidCode: 'Podany kod jest niewłaściwy.',
      forbiddenCharacter: 'Pole zawiera niedozwolone znaki.',
      usernameTaken: 'Nazwa użytkownika jest już w użyciu.',
      invalidKey: 'Klucz jest nieprawidłowy.',
      invalid: 'Pole jest nieprawidłowe.',
      required: 'Pole jest wymagane.',
      maximumLength: 'Maksymalna długość przekroczona.',
      minimumLength: 'Minimalna długość nie została osiągnięta.',
      noSpecialChars: 'Nie wolno używać znaków specjalnych.',
      oneDigit: 'Wymagana jedna cyfra.',
      oneSpecial: 'Wymagany jest znak specjalny.',
      oneUppercase: 'Wymagana jedna duża litera.',
      oneLowercase: 'Wymagana jedna mała litera.',
      portMax: 'Maksymalny numer portu to 65535.',
      endpoint: 'Wpisz prawidłowy punkt końcowy.',
      address: 'Wprowadź poprawny adres.',
      validPort: 'Wprowadź prawidłowy port.',
      validCode: 'Kod powinien mieć 6 cyfr.',
      allowedIps: 'Tylko poprawne adresy IP oraz domeny.',
      startFromNumber: 'Nie może zaczynać się od liczby.',
      repeat: 'Wartości się nie pokrywają.',
      maximumValue: 'Maksymalna wartość {value} przekroczona.',
      minimumValue: 'Minimalna wartość {value} nie osiągnięta.',
      number: 'Wartość musi być liczbą.',
    },
    floatingErrors: {
      title: 'Popraw następujące błędy:',
    },
  },
  components: {
    deviceConfigsCard: {
      cardTitle: 'Konfiguracja lokalizacji',
      messages: {
        copyConfig: 'Konfiguracja skopiowana',
      },
    },
    gatewaysStatus: {
      label: 'Gateways',
      states: {
        error: 'Błąd pobierania statusu',
        loading: 'Pobieranie informacji',
        partial: 'Jeden lub więcej odłączonych',
        connected: 'Połączone',
        disconnected: 'Brak połączenia',
      },
      messages: {
        error: 'Błąd pobierania statusu połączeń gatway',
        deleteError: 'Błąd usuwania gateway',
      },
    },
    noLicenseBox: {
      footer: {
        get: 'Uzyskaj licencję enterprise',
        contact: 'poprzez kontakt:',
      },
    },
  },
  settingsPage: {
    title: 'Ustawienia',
    tabs: {
      smtp: 'SMTP',
      global: 'Globalne',
      ldap: 'LDAP',
      openid: 'OpenID',
    },
    messages: {
      editSuccess: 'Ustawienia zaktualizowane.',
      challengeSuccess: 'Zmieniono wiadomość do podpisu.',
    },
    ldapSettings: {
      title: 'Ustawienia LDAP',
      form: {
        labels: {
          ldap_url: 'URL',
          ldap_bind_username: 'Bind Username',
          ldap_bind_password: 'Bind Password',
          ldap_member_attr: 'Member Attribute',
          ldap_username_attr: 'Username Attribute',
          ldap_user_obj_class: 'User Object Class',
          ldap_user_search_base: 'User Search Base',
          ldap_groupname_attr: 'Groupname Attribute',
          ldap_group_search_base: 'Group Search Base',
          ldap_group_member_attr: 'Group Member Attribute',
          ldap_group_obj_class: 'Group Object Class',
        },
      },
      test: {
        title: 'Test połączenia LDAP',
        messages: {
          error: 'Brak połączenia',
          success: 'Połączono z LDAP',
        },
        submit: 'Test',
      },
    },
    openIdSettings: {
      general: {
        title: 'Ustawienia zewnętrznego OpenID',
        helper:
          'Możesz tu zmienić ogólną mechanikę działania zewnętrznego OpenID w twojej instancji Defguarda.',
        createAccount: {
          label:
            'Automatycznie twórz konta w momencie logowania przez zewnętrznego dostawcę OpenID',
          helper:
            'Jeśli ta opcja jest włączona, Defguard automatycznie tworzy nowe konta dla użytkowników, którzy logują się po raz pierwszy za pomocą zewnętrznego dostawcy OpenID. W innym przypadku konto użytkownika musi zostać najpierw utworzone przez administratora.',
        },
      },
      form: {
        title: 'Ustawienia klienta zewnętrznego OpenID',
        helper:
          'Tutaj możesz skonfigurować ustawienia klienta OpenID z wartościami dostarczonymi przez zewnętrznego dostawcę OpenID.',
        custom: 'Niestandardowy',
        documentation: 'Dokumentacja',
        delete: 'Usuń dostawcę',
        labels: {
          provider: {
            label: 'Dostawca',
            helper:
              'Wybierz swojego dostawcę OpenID. Możesz użyć dostawcy niestandardowego i samodzielnie wypełnić pole URL bazowego.',
          },
          client_id: {
            label: 'ID klienta',
            helper: 'ID klienta dostarczone przez dostawcę OpenID.',
          },
          client_secret: {
            label: 'Sekret klienta',
            helper: 'Sekret klienta dostarczony przez dostawcę OpenID.',
          },
          base_url: {
            label: 'URL bazowy',
            helper:
              'Podstawowy adres URL twojego dostawcy OpenID, np. https://accounts.google.com. Sprawdź naszą dokumentację, aby uzyskać więcej informacji i zobaczyć przykłady.',
          },
        },
      },
    },
    modulesVisibility: {
      header: 'Widoczność modułów',
      helper: `<p>
			Jeśli nie używasz niektórych modułów możesz zmienić ich widoczność
          </p>
          <a href={documentationLink} target="_blank">
					Przeczytaj więcej w dokumentacji.
          </a>`,
      fields: {
        wireguard_enabled: {
          label: 'WireGuard VPN',
        },
        webhooks_enabled: {
          label: 'Webhooks',
        },
        worker_enabled: {
          label: 'YubiBridge',
        },
        openid_enabled: {
          label: 'OpenID connect',
        },
      },
    },
    defaultNetworkSelect: {
      header: 'Domyślny widok sieci',
      helper: `<p>Tutaj możesz zmienić domyślny widok sieci.</p>
          <a href={documentationLink} target="_blank">
					Przeczytaj więcej w dokumentacji.
          </a>`,
      filterLabels: {
        grid: 'Widok siatki',
        list: 'Widok listy',
      },
    },
    web3Settings: {
      header: 'Web3 / Wallet connect',
      fields: {
        signMessage: {
          label: 'Domyślna wiadomość do podpisu',
        },
      },
      controls: {
        save: 'Zapisz zmiany',
      },
    },
    instanceBranding: {
      header: 'Brandowanie instancji',
      form: {
        title: 'Nazwa i logo',
        fields: {
          instanceName: {
            label: 'Nazwa instancji',
            placeholder: 'Defguard',
          },
          mainLogoUrl: {
            label: 'URL logo na stronie logowania',
            helper: '<p>Maksymalna wielkość zdjęcia to 250x100 px.</p>',
            placeholder: 'Domyślny obrazek',
          },
          navLogoUrl: {
            label: 'Menu i nawigacja - małe logo',
            helper: '<p>Maksymalna wielkość zdjęcia to 100x100 px.</p>',
            placeholder: 'Domyślny obrazek',
          },
        },
        controls: {
          restoreDefault: 'Przywróć domyślne',
          submit: 'Zapisz zmiany',
        },
      },
      helper: `
			      <p>
            Tutaj możesz dodać URL swojego logo i nazwę dla swojej instancji defguard;
            będzie ona wyświetlana zamiast defguard.
          </p>
          <a href="{documentationLink}" target="_blank">
					Przeczytaj więcej w dokumentacji.
          </a>
			`,
    },
    smtp: {
      form: {
        title: 'Ustawienia',
        fields: {
          server: {
            label: 'Adres serwera',
            placeholder: 'Adres',
          },
          port: {
            label: 'Port',
            placeholder: 'Port',
          },
          encryption: {
            label: 'Szyfrowanie',
          },
          user: {
            label: 'Użytkownik',
            placeholder: 'Użytkownik',
          },
          password: {
            label: 'Hasło',
            placeholder: 'Hasło',
          },
          sender: {
            label: 'Adres wysyłającego',
            placeholder: 'Adres',
            helper: `
              <p>
                Systemowe wiadomości będą wysyłane z tego adresu, np. no-reply@my-company.com.
              </p>
            `,
          },
        },
        controls: {
          submit: 'Save changes',
        },
      },
      testForm: {
        title: 'Wyślij testowy e-mail',
        fields: {
          to: {
            label: 'Adres',
            placeholder: 'Adres',
          },
        },
        controls: {
          submit: 'Wyślij',
          success: 'E-mail wysłany pomyślnie',
          error: 'Błąd wysyłania e-maila',
        },
      },
      helper: `
        <p>
          Skonfiguruj serwer SMTP do wysyłania wiadomości systemowych do użytkowników.
        </p>
			`,
    },
    enrollment: {
      helper:
        'Rejestracja to proces, w ramach którego nowy użytkownik może samodzielnie aktywować swoje konto, ustawić hasło i skonfigurować urządzenie VPN.',
      vpnOptionality: {
        header: 'Opcjonalność kroku VPN',
        helper:
          'Możesz zdecydować czy dodawanie urządzenia VPN jest obowiązkowym czy opcjonalnym krokiem rejestracji',
      },
      welcomeMessage: {
        header: 'Wiadomość powitalna',
        helper: `
        <p>W tym polu możesz używać Markdown:</p>
        <ul>
          <li>Nagłówki zaczynają się od #</li>
          <li>Użyj asterysków aby uzyskać <i>*kursywę*</i></li>
          <li>Użyj dwóch asterysków aby uzyskać <b>**pogrubienie**</b></li>
        </ul>
        `,
      },
      welcomeEmail: {
        header: 'E-mail powitalny',
        helper: `
        <p>W tym polu możesz używać Markdown:</p>
        <ul>
          <li>Nagłówki zaczynają się od #</li>
          <li>Użyj asterysków aby uzyskać <i>*kursywę*</i></li>
          <li>Użyj dwóch asterysków aby uzyskać <b>**pogrubienie**</b></li>
        </ul>
        `,
      },
      form: {
        controls: {
          submit: 'Zapisz zmiany',
        },
        welcomeMessage: {
          helper:
            'Ta wiadomość będzie pokazywana użytkownikom po zakończeniu rejestracji. Sugerujemy wymienienie w niej istotnych linków oraz krótkie wyjaśnienie kolejnych kroków.',
          placeholder: 'Wpisz wiadomość powitalną',
        },
        welcomeEmail: {
          helper:
            'Ta wiadomość zostanie wysłana do użytkowników po zakończeniu rejestracji. Sugerujemy wymienienie w niej istotnych linków oraz krótkie wyjaśnienie kolejnych kroków. Możesz użyć tej samej treści co w wiadomości powitalnej.',
          placeholder: 'Wpisz e-mail powitalny',
        },
        welcomeEmailSubject: {
          label: 'Temat',
        },
        useMessageAsEmail: {
          label: 'Taki sam jak wiadomość powitalna',
        },
      },
    },
  },
  openidOverview: {
    pageTitle: 'Aplikacje OpenID',
    search: {
      placeholder: 'Znajdź aplikacje',
    },
    filterLabels: {
      all: 'Wszystkie aplikacje',
      enabled: 'Włączone',
      disabled: 'Wyłączone',
    },
    clientCount: 'Wszystkie aplikacje',
    addNewApp: 'Dodaj aplikację',
    list: {
      headers: {
        name: 'Nazwa',
        status: 'Status',
        actions: 'Akcję',
      },
      editButton: {
        edit: 'Edytuj aplikację',
        delete: 'Usuń aplikację',
        disable: 'Wyłącz',
        enable: 'Włącz',
        copy: 'Skopuj ID',
      },
      status: {
        enabled: 'Włączona',
        disabled: 'Wyłączona',
      },
    },
    messages: {
      noLicenseMessage: 'Nie masz licencji dla tej funkcjonalności.',
      noClientsFound: 'Nie znaleziono żadnych wyników.',
      copySuccess: 'ID skopiowane',
    },
    deleteApp: {
      title: 'Usuń aplikację',
      message: 'Czy chcesz usunąć aplikację {appName} ?',
      submit: 'Usuń aplikację',
      messages: {
        success: 'Aplikacja usunięta.',
      },
    },
    enableApp: {
      messages: {
        success: 'Aplikacja włączona',
      },
    },
    disableApp: {
      messages: {
        success: 'Aplikacja wyłączona',
      },
    },
    modals: {
      openidClientModal: {
        title: {
          addApp: 'Dodaj aplikację',
          editApp: 'Edytuj aplikację: {appName}',
        },
        scopes: 'Zakresy:',
        messages: {
          clientIdCopy: 'Client ID zostało skopiowane.',
          clientSecretCopy: 'Client secret zostało skopiowane.',
        },
        form: {
          messages: {
            successAdd: 'Aplikacja utworzona.',
            successModify: 'Aplikacja zmodyfikowana.',
          },
          error: {
            urlRequired: 'URL jest wymagany.',
            validUrl: 'URL musi być poprawny.',
            scopeValidation: 'Musi mieć co najmniej jeden zakres.',
          },
          fields: {
            name: {
              label: 'Nazwa aplikacji',
            },
            redirectUri: {
              label: 'Przekierowujący URL {count}',
              placeholder: 'https://example.com/redirect',
            },
            openid: {
              label: 'OpenID',
            },
            profile: {
              label: 'Profil',
            },
            email: {
              label: 'E-mail',
            },
            phone: {
              label: 'Telefon',
            },
            groups: {
              label: 'Grupy',
            },
          },
          controls: {
            addUrl: 'Dodaj URL',
          },
        },
        clientId: 'Client ID',
        clientSecret: 'Client secret',
      },
    },
  },
  webhooksOverview: {
    pageTitle: 'Webhooki',
    search: {
      placeholder: 'Znajdź webhooki po adresie URL',
    },
    filterLabels: {
      all: 'Wszystkie webhooki',
      enabled: 'Włączone',
      disabled: 'Wyłączone',
    },
    webhooksCount: 'Wszystkie webhooki',
    addNewWebhook: 'Dodaj webhook',
    noWebhooksFound: 'Nie znaleziono żadnych webhooków',
    list: {
      headers: {
        name: 'Nazwa',
        description: 'Opis',
        status: 'Status',
        actions: 'Akcję',
      },
      editButton: {
        edit: 'Edytuj',
        delete: 'Usuń webhook',
        disable: 'Wyłącz',
        enable: 'Włącz',
      },
      status: {
        enabled: 'Włączony',
        disabled: 'Wyłączony',
      },
    },
  },
  provisionersOverview: {
    pageTitle: 'Provisionery',
    search: {
      placeholder: 'Wyszukaj provisionera',
    },
    filterLabels: {
      all: 'Wszystkie',
      available: 'Dostępne',
      unavailable: 'Niedostępne',
    },
    provisionersCount: 'Wszystkie provisionery',
    noProvisionersFound: 'Nie znaleziono provisionerów.',
    noLicenseMessage: 'Nie masz licencji na tę funkcję.',
    provisioningStation: {
      header: 'Stacja provisionująca YubiKey',
      content: `Aby móc sprovisionować YubiKeya, należy najpierw skonfigurować
        fizyczną maszynę z gniazdem USB. Uruchom podane polecenie na wybranej maszynie
        aby zarejestrować maszynę i rozpocząć generowanie kluczy.`,
      tokenCard: {
        title: 'Token autoryzacyjny',
      },
      dockerCard: {
        title: 'Przykład Docker',
      },
    },
    list: {
      headers: {
        name: 'Nazwa',
        ip: 'Adres IP',
        status: 'Status',
        actions: 'Akcję',
      },
      editButton: {
        delete: 'Usuń provisionera',
      },
      status: {
        available: 'Dostępny',
        unavailable: 'Niedostępny',
      },
    },
    messages: {
      copy: {
        command: 'Komenda skopiowa',
        token: 'Token skopiowany',
      },
    },
  },
  openidAllow: {
    header: '{name} chciałby:',
    scopes: {
      openid: 'Użyć danych z twojego profilu do przyszłych logowań.',
      profile:
        'Poznać podstawowe informacje z twojego profilu, takie jak login, imię itp',
      email: 'Poznać twój adres e-mail.',
      phone: 'Poznać twój numer telefonu.',
      groups: 'Poznać twoje grupy.',
    },
    controls: {
      accept: 'Akceptuj',
      cancel: 'Anuluj',
    },
  },
  networkOverview: {
    pageTitle: 'Przegląd lokalizacji',
    controls: {
      editNetworks: 'Edycja lokalizacji',
      selectNetwork: {
        placeholder: 'Oczekiwanie na lokalizacje',
      },
    },
    filterLabels: {
      grid: 'Widok siatki',
      list: 'Widok listy',
    },
    stats: {
      currentlyActiveUsers: 'Obecnie aktywni użytkownicy',
      currentlyActiveDevices: 'Obecnie aktywne urządzenia',
      activeUsersFilter: 'Aktywni użytkownicy w {hour}H',
      activeDevicesFilter: 'Aktywne urządzenia w {hour}H',
      totalTransfer: 'Całkowity transfer:',
      activityIn: 'Aktywność w {hour}H',
      in: 'Przychodzący:',
      out: 'Wychodzący:',
      gatewayDisconnected: 'Gateway rozłączony',
    },
  },
  connectedUsersOverview: {
    pageTitle: 'Podłączeni użytkownicy',
    noUsersMessage: 'Obecnie nie ma żadnych podłączonych użytkowników',
    userList: {
      username: 'Nazwa użytkownika',
      device: 'Urządzenia:',
      connected: 'Połączony:',
      deviceLocation: 'Lokacja urządzenia',
      networkUsage: 'Użycie sieci',
    },
  },
  networkPage: {
    pageTitle: 'Edycja lokalizacji',
    addNetwork: '+ Dodaj lokalizację',
    controls: {
      networkSelect: {
        label: 'Wybór lokalizacji',
      },
    },
  },
  activityOverview: {
    header: 'Strumień aktywności',
    noData: 'Obecnie nie wykryto żadnej aktywności',
  },
  networkConfiguration: {
    messages: {
      delete: {
        error: 'Błąd podczas próby usunięcia lokalizacji',
        success: 'Lokalizacja usunięta',
      },
    },
    header: 'Konfiguracja lokalizacji',
    importHeader: 'Import lokalizacji',
    form: {
      helpers: {
        address:
          'Od tego adresu będzie stworzona sieć VPN, np. 10.10.10.1/24 (sieć VPN będzie: 10.10.10.0/24)',
        gateway:
          'Adres publiczny Gatewaya, używany przez użytkowników VPN do łączenia się.',
        dns: 'Określ resolwery DNS, które mają odpytywać, gdy interfejs WireGuard jest aktywny.',
        allowedIps: 'Lista adresów/masek, które powinny być routowane przez sieć VPN.',
        allowedGroups:
          'Domyślnie wszyscy użytkownicy będą mogli połączyć się z tą lokalizacją. Jeżeli chcesz ogranicznyć dostęp do tej lokalizacji do wybranej grupy użytkowników, wybierz ją poniżej.',
      },
      messages: {
        networkModified: 'Lokalizacja zmodyfikowana',
        networkCreated: 'Lokalizacja utworzona',
      },
      fields: {
        name: {
          label: 'Nazwa lokalizacji',
        },
        address: {
          label: 'Adres i maska sieci VPN',
        },
        endpoint: {
          label: 'Adres gatewaya',
        },
        allowedIps: {
          label: 'Dozwolone adresy IP',
        },
        port: {
          label: 'Port Gatewaya',
        },
        dns: {
          label: 'DNS',
        },
        allowedGroups: {
          label: 'Dozwolone grupy',
          placeholder: 'Wszystkie grupy',
        },
        mfa_enabled: {
          label: 'Wymagaj MFA dla tej lokalizacji',
        },
        keepalive_interval: {
          label: 'Utrzymanie połączenia [sekundy]',
        },
        peer_disconnect_threshold: {
          label: 'Próg rozłączania [sekundy]',
        },
      },
      controls: {
        submit: 'Zapisz zmiany',
        cancel: 'Wróć',
        delete: 'Usuń lokalizację',
      },
    },
  },
  gatewaySetup: {
    header: 'Uruchomienie serwera gateway',
    card: {
      title: 'Komenda Dockera uruchamiająca serwer gateway',
    },
    controls: {
      status: 'Sprawdź status połączenia',
    },
    messages: {
      runCommand: `
          <p>
            Defguard wymaga uruchomienia serwera gateway w celu kontrolowania VPN.
            Szczegóły znajdziesz w <a href="{setupGatewayDocs}" target="_blank">dokumentacji</a>.
            Istnieje wiele sposobów na uruchomienie serwera gateway, poniższy przykład używa technologii Docker,
            więcej przykładów znajdziesz w <a href="{setupGatewayDocs}" target="_blank">dokumentacji</a>.
          </p>`,
      createNetwork: `
          <p>
            Utwórz sieć przed uruchomieniem procesu gateway.
          </p>`,
      noConnection: `<p>Brak połączenia proszę uruchom poniższą komendę.</p>`,
      connected: `<p>Gateway połączony.</p>`,
      statusError: 'Nie udało się uzyskać statusu',
    },
  },
  loginPage: {
    pageTitle: 'Wprowadź swoje dane logowania',
    callback: {
      return: 'Powrót do logowania',
      error: 'Wystąpił błąd podczas logowania przez zewnętrznego dostawcę OpenID',
    },
    mfa: {
      title: 'Autoryzacja dwuetapowa.',
      controls: {
        useAuthenticator: 'Zamiast tego użyj aplikacji Authenticator',
        useWallet: 'Zamiast tego użyj swojego portfela kryptowalutowego',
        useWebauthn: 'Zamiast tego użyj klucza bezpieczeństwa',
        useRecoveryCode: 'Zamiast tego użyj kodu odzyskiwania',
        useEmail: 'Zamiast tego użyj e-mail',
      },
      email: {
        header: 'Użyj kodu wysłanego na e-mail aby kontynuować',
        form: {
          controls: {
            resendCode: 'Wyślij kod ponownie',
          },
          labels: {
            code: 'Kod',
          },
        },
      },
      totp: {
        header:
          'Użyj kodu z aplikacji uwierzytelniającej i kliknij przycisk, aby kontynuować',
        form: {
          fields: {
            code: {
              placeholder: 'Wprowadź kod uwierzytelniający',
            },
          },
          controls: {
            submit: 'Użyj kodu uwierzytelniającego',
          },
        },
      },
      recoveryCode: {
        header:
          'Wpisz jeden z aktywnych kodów odzyskiwania i kliknij przycisk, aby się zalogować.',
        form: {
          fields: {
            code: {
              placeholder: 'Kod odzyskiwania',
            },
          },
          controls: {
            submit: 'Użyj kodu odzyskiwania',
          },
        },
      },
      wallet: {
        header:
          'Użyj portfela kryptowalutowego, aby się zalogować, proszę podpisać wiadomość w aplikacji portfelowej lub rozszerzeniu.',
        controls: {
          submit: 'Użyj swojego portfela',
        },
        messages: {
          walletError: 'Portfel został rozłączony podczas procesu podpisywania.',
          walletErrorMfa:
            'Portfel nie jest autoryzowany do logowania MFA. Proszę użyć autoryzowanego portfela.',
        },
      },
      webauthn: {
        header: 'Gdy jesteś gotowy do uwierzytelnienia, naciśnij przycisk poniżej.',
        controls: {
          submit: 'Użyj klucza bezpieczeństwa',
        },
        messages: {
          error: 'Nie udało się odczytać klucza. Proszę spróbować ponownie.',
        },
      },
    },
  },
  wizard: {
    completed: 'Sieć skonfigurowa',
    configuration: {
      successMessage: 'Sieć utworzona',
    },
    navigation: {
      top: 'Konfiguracja sieci',
      titles: {
        welcome: 'Konfiguracja sieci',
        choseNetworkSetup: 'Wybierz tryb konfiguracji',
        importConfig: 'Importuj istnijącą sieć',
        manualConfig: 'Konfiguracja sieci',
        mapDevices: 'Mapowanie importowanych urządzeń',
      },
      buttons: {
        next: 'Dalej',
        back: 'Wróć',
      },
    },
    welcome: {
      header: 'Witaj w asystencie konfiguracji lokalizacji!',
      sub: 'Zanim zaczniesz, musisz wybrać tryb konfiguracji. Ikony <React> zawierają przydane informacje.',
      button: 'Zacznij konfigurację',
    },
    deviceMap: {
      messages: {
        crateSuccess: 'Urządzenie dodane',
        errorsInForm: 'Uzupełnij oznaczone pola',
      },
      list: {
        headers: {
          deviceName: 'Nazwa',
          deviceIP: 'IP',
          user: 'Użytkownik',
        },
      },
    },
    wizardType: {
      manual: {
        title: 'Manualny',
        description: 'Ręczna konfiguracja sieci WireGuard',
      },
      import: {
        title: 'Import',
        description: 'Import z pliku konfiguracyjnego WireGuard',
      },
      createNetwork: 'Utwórz sieć WireGuard',
    },
    common: {
      select: 'Wybierz',
    },
    locations: {
      form: {
        name: 'Nazwa',
        ip: 'Adres IP',
        user: 'Użytkownik',
        fileName: 'Plik',
        selectFile: 'Wybierz plik',
        messages: { devicesCreated: 'Urządzenia utworzone.' },
        validation: { invalidAddress: 'Nieprawidłowy adres.' },
      },
    },
  },
  layout: {
    select: {
      addNewOptionDefault: 'Dodaj +',
    },
  },
  redirectPage: {
    title: 'Zostałeś zalogowany',
    subtitle: 'Wkrótce zostaniesz przekierowany...',
  },
  enrollmentPage: {
    title: 'Rejestracja',
    controls: {
      default: 'Domyślne',
      save: 'Zapisz zmiany',
    },
    messages: {
      edit: {
        error: 'Zapis nieudany',
        success: 'Zapisano zmiany',
      },
    },
    settings: {
      welcomeMessage: {
        title: 'Powitalna wiadomość',
        messageBox: 'Ta informacja będzie wyświetlona w końcowym kroku rejestracj',
      },
      welcomeEmail: {
        subject: {
          label: 'Temat wiadomości',
        },
        title: 'Powitalny e-mail',
        messageBox: 'Ta informacja będzie wysłana gdy użytkownik zakończy rejestrację.',
        controls: {
          duplicateWelcome: 'Identyczna jak wiadomość powitalna',
        },
      },
      vpnOptionality: {
        title: 'Opcjonalność kroku VPN',
        select: {
          options: {
            optional: 'Opcjonalny',
            mandatory: 'Obowiązkowy',
          },
        },
      },
    },
    messageBox:
      'Proces rejestracji pozwala użytkownikowi na potwierdzenie swoich informacji, ustawienie hasła oraz skonfigurowanie VPN na swoim urządzeniu. Tutaj możesz skonfigurować ten proces.',
  },
  supportPage: {
    title: 'Wsparcie',
    modals: {
      confirmDataSend: {
        title: 'Potwierdź przekazanie danych',
        submit: 'Wyślij',
        subTitle:
          'Potwierdź przesłanie danych diagnostycznych. Żadne poufne dane nie zostaną przesłane. (Klucze WireGuard, adresy e-mail, itp.)',
      },
    },
    debugDataCard: {
      title: 'Dane wsparcia technicznego',
      body: `
Jeśli potrzebujesz pomocy lub zostałeś poproszony przez nasz zespół o utworzenie danych wsparcia technicznego (np. na naszym kanale Matrix: **#defguard-support:teonite.com**), masz dwie opcje:
* Możesz skonfigurować ustawienia SMTP i kliknąć: "Wyślij dane wsparcia technicznego".
* Lub kliknąć "Pobierz dane wsparcia technicznego" i stworzyć zlecenie w naszym repozytorium GitHub załączając te pliki.
`,
      downloadSupportData: 'Pobierz dane wsparcia technicznego',
      downloadLogs: 'Pobierz dzienniki',
      sendMail: 'Wyślij e-mail',
      mailSent: 'E-mail wysłany',
      mailError: 'Błąd wysyłania e-mail',
    },

    supportCard: {
      title: 'Wsparcie',
      body: `
Przed zgłoszeniem problemów na GitHub należy zapoznać z dokumentacją dostępną na [defguard.gitbook.io/defguard](https://defguard.gitbook.io/defguard/)

Aby zgłosić:
* Problem - przejdź do [GitHub](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=bug&template=bug_report.md&title=)
* Prośbę o nową funkcjonalność - przejdź do [GitHub](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=feature&template=feature_request.md&title=)

W przypadku innych zgłoszeń skontaktuj się z nami: support@defguard.net
`,
    },
  },
};

export default pl;

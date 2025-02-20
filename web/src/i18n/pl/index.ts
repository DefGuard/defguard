import { deepmerge } from 'deepmerge-ts';

import en from '../en';
import { Translation } from '../i18n-types';

const translation: Translation = {
  common: {
    conditions: {
      and: 'I',
      equal: 'Równy',
      or: 'Albo',
    },
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
      dismiss: 'Odrzuć',
      show: 'Pokaż',
    },
    key: 'Klucz',
    name: 'Nazwa',
    noData: 'Brak danych',
    unavailable: 'Niedostępne',
    notSet: 'Nieustawione',
  },
  messages: {
    error: 'Wystąpił błąd.',
    success: 'Operacja zakończyła się sukcesem',
    errorVersion: 'Nie udało się uzyskać wersji aplikacji.',
    details: 'Szczegóły:',
    clipboard: {
      success: 'Skopiowano do schowka',
      error: 'Schowek nie jest dostępny',
    },
    insecureContext: 'Kontekst nie jest bezpieczny',
  },
  modals: {
    upgradeLicenseModal: {
      enterprise: {
        title: 'Podnieś do Enterprise',
        //md
        subTitle: `Został przekroczony limit użytkowników, urządzeń lub sieci, a ta funkcjonalność jest dostępna tylko w wersji **enterprise**. Aby użyć tej funkcjonalności, należy zakupić lub podnieść obecną licencję enterprise.`,
      },
      limit: {
        title: 'Podnieś',
        //md
        subTitle: `
        **Osiągnięto limit** funkcjonalności. Aby **[ zarządzać większą liczbą lokalizacji/użytkowników/urządzeń ]** wymagany jest zakup licencji Enterprise.
        `,
      },
      //md
      content: `
Aby dowiedzieć się więcej o:
- Automatyczniej synchronizacji klientów w czasie rzeczywistym
- Zewnętrznym SSO
- Kontrolowaniu działania klientów VPN

Pełna lista funkcjonalności enterprise: [https://docs.defguard.net/enterprise/all-enteprise-features](https://docs.defguard.net/enterprise/all-enteprise-features)</br>
Informacja o licencjonowaniu: [https://docs.defguard.net/enterprise/license](https://docs.defguard.net/enterprise/license)
      `,
      controls: {
        cancel: 'Może później',
        confirm: 'Wszystkie plany Enterprise',
      },
    },
    standaloneDeviceEnrollmentModal: {
      title: 'Network device token',
      toasters: {
        error: 'Token generation failed.',
      },
    },
    standaloneDeviceConfigModal: {
      title: 'Konfiguracja urządzenia sieciowego',
      cardTitle: 'Konfiguracja',
      toasters: {
        getConfig: {
          error: 'Nie udało się pobrać konfiguracji urządzenia.',
        },
      },
    },
    editStandaloneModal: {
      title: 'Edycja urządzenia sieciowego',
      toasts: {
        success: 'Urządzenia zostało zmienione',
        failure: 'Nie udało się zmienić urządzenia.',
      },
    },
    deleteStandaloneDevice: {
      title: 'Usuń urządzenie sieciowe',
      content: 'Urządzenie {name: string} zostanie usunięte.',
      messages: {
        success: 'Urządzenie zostało usunięte',
        error: 'Nie udało się usunąć urządzenia.',
      },
    },
    addStandaloneDevice: {
      toasts: {
        deviceCreated: 'Urządzenie zostało dodane',
        creationFailed: 'Urządzenie nie mogło być dodane.',
      },
      infoBox: {
        setup:
          'Tu można dodać definicje lub utworzyć konfiguracje dla urządzeń, które można podłączyć do sieci VPN. Dostępne są jedynie lokalizacje bez uwierzytelniania wieloskładnikowego (MFA), ponieważ póki co ta funkcjonalność jest dostępna tylko w kliencie Defguard Desktop.',
      },
      form: {
        submit: 'Dodaj urządzenie',
        labels: {
          deviceName: 'Nazwa urządzenia',
          location: 'Położenie',
          assignedAddress: 'Przydzielony adres IP',
          description: 'Opis',
          generation: {
            auto: 'Utwórz parę kluczy',
            manual: 'Własny klucz publiczny',
          },
          publicKey: 'Podaj swój klucz publiczny',
        },
      },
      steps: {
        method: {
          title: 'Wybierz preferowaną metodę',
          cards: {
            cli: {
              title: 'Klient Defguard CLI',
              subtitle:
                'Używając defguard-cli urządznie zostanie automatycznie skonfigurowane.',
              docs: 'Pobieranie i dokumentacja klienta Defguard CLI',
            },
            manual: {
              title: 'Ręczny klient WireGuard',
              subtitle:
                'Jeżeli Twoje urządzenie nie wspiera naszych programów CLI, zawsze można utworzyć plik konfiguracyjny WireGuard i skonfigurowć je ręcznie - ale w takim przypadku uaktualnienia lokalizacji VPN będą wymagały ręcznych zmian w urządzeniu.',
            },
          },
        },
        manual: {
          title: 'Dodaj nowe urządzenie VPN używając klienta WireGuard',
          finish: {
            messageTop:
              'Pobierz podany plik konfiguracyjny na urządzeniu i zaimportuj go do klienta VPN żeby zakończyć jego konfigurowanie.',
            ctaInstruction:
              'Użyj podanego niżej pliku konfiguracyjnego skanując kod QR lub importując go jako plik w aplikacji WireGuard na urządzeniu.',
            // MD
            warningMessage: `
            Należy pamiętać, że Defguard **nie przechowuje kluczy prywatnych**. Para kluczy (publiczny i prywatny) zostanie bezpiecznie utworzona w przeglądarce, ale jedynie klucz publiczny zostanie zapisany w bazie danych Defguard. Proszę pobrać utworzoną konfigurację zawierającą klucz prywatny dla urządzenia, gdyż nie będzie ona później dostępna.
            `,
            actionCard: {
              title: 'Konfiguracja',
            },
          },
        },
        cli: {
          title: 'Dodaj urządzenie używając klienta Defguard CLI',
          finish: {
            topMessage:
              'Najpierw pobierz klienta Defguard CLI i zainstaluj go na serwerze.',
            downloadButton: 'Pobierz klienta Defguard CLI',
            commandCopy: 'Skopiuj i wklej to polecenie w terminalu na urządzeniu',
          },
          setup: {
            stepMessage:
              'Tu można dodać definicje lub utworzyć konfiguracje dla urządzeń, które mogą łączyć się do sieci VPN. Tutaj dostępne są jedynie lokalizacje bez uwierzytelniania wieloskładnikowego (MFA) ponieważ póki co MFA jest wspierane jedynie w kliencie Defguard Desktop.',
            form: {
              submit: 'Dodaj urządzenie',
            },
          },
        },
      },
    },
    updatesNotification: {
      header: {
        criticalBadge: 'Aktualizacja krytyczna',
        newVersion: 'Nowa wersja {version}',
        title: 'Aktualizacja dostępna',
      },
      controls: {
        visitRelease: 'Zobacz stronę aktualizacji',
      },
    },
    updatesNotificationToaster: {
      title: 'Nowa wersja dostępna {version}',
      controls: {
        more: 'Zobacz co nowego',
      },
    },
    addGroup: {
      groupName: 'Nazwa grupy',
      searchPlaceholder: 'Szukaj',
      selectAll: 'Zaznacz wszystkich',
      submit: 'Stwórz grupę',
      title: 'Dodaj grupę',
      groupSettings: 'Ustawienia grupy',
      adminGroup: 'Grupa administratorska',
    },
    editGroup: {
      groupName: 'Nazwa grupy',
      searchPlaceholder: 'Szukaj',
      selectAll: 'Zaznacz wszystkich',
      submit: 'Zmień grupę',
      title: 'Edytuj grupę',
      groupSettings: 'Ustawienia grupy',
      adminGroup: 'Grupa administratorska',
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
      copyPath: 'Kopiuj ścieżkę TOTP',
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
            label: 'Nazwisko',
          },
          phone: {
            placeholder: 'Telefon',
            label: 'Telefon',
          },
          enableEnrollment: {
            label: 'Użyj zdalnej rejestracji',
            link: '<a href="https://docs.defguard.net/help/enrollment" target="_blank">więcej informacji tutaj</a>',
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
      client: `Pobierz klienta defguard <a href="https://defguard.net/download" target="_blank">tutaj</a>, a następnie postępuj zgodnie z <a href="https://docs.defguard.net/help/configuring-vpn/add-new-instance" target="_blank">instrukcją</a> w celu jego konfiguracji.`,
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
      warningModals: {
        title: 'Ostrzeżenie',
        content: {
          usernameChange: `Zmiana nazwy użytkownika ma znaczący wpływ na usługi, do których użytkownik zalogował się za pomocą Defguard. Po zmianie nazwy użytkownika użytkownik może stracić do nich dostęp (ponieważ nie będą go rozpoznawać). Czy na pewno chcesz kontynuować?`,
          emailChange: `Jeśli korzystasz z zewnętrznych dostawców OpenID Connect (OIDC) do uwierzytelniania użytkowników, zmiana adresu e-mail użytkownika może mieć wpływ na jego możliwość zalogowania się do Defguarda. Czy na pewno chcesz kontynuować?`,
        },
        buttons: {
          proceed: 'Proceed',
          cancel: 'Cancel',
        },
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
        labels: {
          totp: 'Hasła jednorazowe oparte na czasie',
          webauth: 'Klucze bezpieczeństwa',
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
    apiTokens: {
      header: 'API Tokeny użytkownika',
      addToken: 'Dodaj nowy API Token',
      tokensList: {
        common: {
          rename: 'Zmień nazwę',
          token: 'Token',
          copy: 'Skopiuj',
          delete: 'Usuń',
          createdAt: 'Utworzono',
        },
      },
      deleteModal: {
        title: 'Usuń API Token',
        confirmMessage: 'API token {name: string} zostanie trwale usunięty.',
      },
      addModal: {
        header: 'Dodaj nowy API Token',
        tokenForm: {
          placeholders: {
            name: 'Nazwa API Tokena',
          },
          labels: {
            name: 'Nazwa',
          },
          submit: 'Dodaj API token',
        },
        copyToken: {
          warningMessage:
            'Skopiuj poniższy API token teraz. Nie będzie on dostępny w późniejszym czasie.',
          header: 'Skopiuj nowy API Token',
        },
        messages: {
          tokenAdded: 'API token dodany.',
          genericError: 'Nie udało się dodać API tokena. Spróbuj ponownie później.',
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
    copyright: 'Copyright ©2023-2024',
    version: {
      open: 'Wersja aplikacji: {version}',
      closed: 'v{version}',
    },
  },
  form: {
    download: 'Pobierz',
    copy: 'Kopiuj',
    saveChanges: 'Zapisz zmiany',
    submit: 'Zapisz',
    login: 'Zaloguj się',
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
      endpoint: 'Wpisz poprawny adres.',
      address: 'Wprowadź poprawny adres.',
      addressNetmask: 'Wprowadź poprawny adres IP oraz maskę sieci.',
      validPort: 'Wprowadź prawidłowy port.',
      validCode: 'Kod powinien mieć 6 cyfr.',
      allowedIps: 'Tylko poprawne adresy IP oraz domeny.',
      startFromNumber: 'Nie może zaczynać się od liczby.',
      repeat: 'Wartości się nie pokrywają.',
      maximumValue: 'Maksymalna wartość {value} przekroczona.',
      minimumValue: 'Minimalna wartość {value} nie osiągnięta.',
      tooManyBadLoginAttempts:
        'Zbyt duża ilość nieprawidłowego logowania. Spróbuj ponownie za kilka minut.',
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
      enterprise: 'Funkcjonalności enterprise',
    },
    messages: {
      editSuccess: 'Ustawienia zaktualizowane.',
      challengeSuccess: 'Zmieniono wiadomość do podpisu.',
    },
    enterpriseOnly: {
      title: 'Ta funkcja jest dostępna tylko w wersji Defguard Enterprise',
      currentExpired: 'Twoja obecna licencja wygasła.',
      subtitle: 'Aby uzyskać więcej informacji, odwiedź naszą ',
      website: 'stronę internetową',
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
        delete: 'Usuń konfigurację',
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
        title: 'Ogólne ustawienia zewnętrznego OpenID',
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
        none: 'Brak',
        documentation: 'Dokumentacja',
        delete: 'Usuń dostawcę',
        directory_sync_settings: {
          title: 'Ustawienia synchronizacji katalogu',
          helper:
            'Synchronizacja katalogu pozwala na automatyczną synchronizację grup użytkowników i ich statusu na podstawie zewnętrznego dostawcy.',
          notSupported: 'Synchronizacja katalogu nie jest obsługiwana dla tego dostawcy.',
          connectionTest: {
            success: 'Połączenie zakończone sukcesem.',
            error: 'Wystąpił błąd podczas próby połączenia:',
          },
        },
        selects: {
          synchronize: {
            all: 'Wszystko',
            users: 'Użytkownicy',
            groups: 'Grupy',
          },
          behavior: {
            keep: 'Zachowaj',
            disable: 'Dezaktywuj',
            delete: 'Usuń',
          },
        },
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
          display_name: {
            label: 'Wyświetlana nazwa',
            helper:
              'Nazwa dostawcy OpenID, która będzie wyświetlana na przycisku logowania. Jeśli zostawisz to pole puste, przycisk będzie miał tekst "Zaloguj przez OIDC".',
          },
          enable_directory_sync: {
            label: 'Włącz synchronizację katalogu',
          },
          sync_target: {
            label: 'Synchronizuj',
            helper:
              'Co będzie synchronizowane z zewnętrznym dostawcą OpenID. Możesz wybrać pomiędzy synchronizacją statusu użytkowników, ich przynależności do grup lub synchronizacją obu.',
          },
          sync_interval: {
            label: 'Interwał synchronizacji',
            helper: 'Odstęp czasu w sekundach pomiędzy synchronizacjami katalogu.',
          },
          user_behavior: {
            label: 'Zachowanie kont użytkowników',
            helper:
              'Wybierz jak postępować z kontami użytkowników, które nie znajdują się w katalogu zewnętrznego dostawcy. Możesz wybrać między zachowaniem ich, dezaktywacją lub całkowitym usunięciem.',
          },
          admin_behavior: {
            label: 'Zachowanie kont administratorów',
            helper:
              'Wybierz, jak postępować z kontami administratorów Defguard, które nie znajdują się w katalogu zewnętrznego dostawcy. Możesz wybrać między zachowaniem ich, dezaktywacją lub całkowitym usunięciem.',
          },
          admin_email: {
            label: 'E-mail administratora',
            helper:
              'Adres e-mail konta, za pośrednictwem którego będzię odbywać się synchronizacja, np. e-mail konta osoby, która skonfigurowała konto usługi Google. Więcej szczegółów możesz znaleźć w naszej dokumentacji.',
          },
          service_account_used: {
            label: 'Używane konto usługi',
            helper:
              'Obecnie używane konto usługi Google do synchronizacji. Możesz je zmienić, przesyłając nowy plik klucza konta usługi.',
          },
          service_account_key_file: {
            label: 'Plik klucza konta usługi',
            helper:
              'Prześlij nowy plik klucza konta usługi, aby ustawić konto usługi używane do synchronizacji. UWAGA: Przesłany plik nie będzie widoczny po zapisaniu ustawień i ponownym załadowaniu strony, ponieważ jego zawartość jest poufna i nie jest przesyłana z powrotem do panelu.',
            uploaded: 'Przesłany plik',
            uploadPrompt: 'Prześlij plik klucza konta usługi',
          },
          okta_client_id: {
            label: 'ID klienta synchronizacji Okta',
            helper: 'ID klienta dla aplikacji synchronizacji Okta.',
          },
          okta_client_key: {
            label: 'Klucz prywatny klienta synchronizacji Okta',
            helper:
              'Klucz prywatny dla aplikacji synchronizacji Okta w formacie JWK. Klucz nie jest wyświetlany ponownie po wgraniu.',
          },
        },
      },
    },
    modulesVisibility: {
      header: 'Widoczność modułów',
      helper: `<p>
			Jeśli nie używasz niektórych modułów, możesz zmienić ich widoczność
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
    license: {
      header: 'Funkcje enterprise',
      helpers: {
        enterpriseHeader: {
          text: 'Tutaj możesz zarządzać swoją licencją Defguard Enterprise.',
          link: 'By dowiedzieć się więcej, odwiedź naszą stronę.',
        },
        licenseKey: {
          text: 'Wprowadź poniżej klucz licencyjny Defguard Enterprise. Powinieneś otrzymać go na swoją skrzynkę e-mailową po zakupie licencji.',
          link: 'Licencję możesz zakupić tutaj.',
        },
      },
      form: {
        title: 'Licencja',
        fields: {
          key: {
            label: 'Klucz licencji',
            placeholder: 'Klucz licencji dla twojej instancji Defguard',
          },
        },
      },
      licenseInfo: {
        title: 'Informacje o licencji',
        noLicense: 'Brak ważnej licencji',
        licenseNotRequired:
          "<p>Posiadasz dostęp do tej funkcji enterprise, ponieważ nie przekroczyłeś jeszcze żadnych limitów. Sprawdź <a href='https://docs.defguard.net/enterprise/license'>dokumentację</a>, aby uzyskać więcej informacji.</p>",
        types: {
          subscription: {
            label: 'Subskrypcja',
            helper: 'Subskrypcja automatycznie odnawiana cyklicznie',
          },
          offline: {
            label: 'Offline',
            helper: 'Licencja ważna do daty wygaśnięcia, odnawiana ręcznie',
          },
        },
        fields: {
          status: {
            label: 'Status',
            active: 'Aktywna',
            expired: 'Wygasła',
            subscriptionHelper:
              'Licencja w formie subskrypcji jest ważna przez pewien czas po dacie wygaśnięcia, by uwzględnić możliwe opóźnienia w automatycznej płatności.',
          },
          type: {
            label: 'Typ',
          },
          validUntil: {
            label: 'Ważna do',
          },
        },
      },
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
          submit: 'Zapisz zmiany',
        },
      },
      delete: 'Usuń konfigurację',
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
    enterprise: {
      header: 'Funkcjonalności Enterprise',
      helper: '<p>Tutaj możesz zmienić ustawienia enterprise.</p>',
      fields: {
        deviceManagement: {
          label: 'Zablokuj możliwość zarządzania urządzeniami przez użytkowników',
          helper:
            'Kiedy ta opcja jest włączona, tylko użytkownicy w grupie "Admin" mogą zarządzać urządzeniami w profilu użytkownika',
        },
        disableAllTraffic: {
          label: 'Zablokuj możliwość przekierowania całego ruchu przez VPN',
          helper:
            'Kiedy ta opcja jest włączona, użytkownicy nie będą mogli przekierować całego ruchu przez VPN za pomocą klienta Defguard.',
        },
        manualConfig: {
          label: 'Wyłącz manualną konfigurację WireGuard',
          helper:
            'Kiedy ta opcja jest włączona, użytkownicy nie będą mogli pobrać ani wyświetlić danych do manualnej konfiguracji WireGuard. Możliwe będzie wyłącznie skonfigurowanie klienta Defguard.',
        },
      },
    },
    gatewayNotifications: {
      smtpWarning:
        'Aby włączyć powiadomienia o rozłączeniu należy najpierw skonfigurować serwer SMTP',
      header: 'Powiadomienia Gateway',
      helper: "<p>Tutaj możesz włączyć powiadomienia o rozłączeniu się Gateway'a.</p>",
      form: {
        submit: 'Zapisz zmiany',
        fields: {
          disconnectNotificationsEnabled: {
            label: 'Włącz powiadomienia o rozłączeniu',
            help: "Wyślij powiadomienie do administratorów po rozłączeniu się Gateway'a",
          },
          inactivityThreshold: {
            label: 'Czas nieaktywności [minuty]',
            help: 'Czas (w minutach), który musi upłynąć od rozłączenia zanim zostanie wysłane powiadomienie',
          },
          reconnectNotificationsEnabled: {
            label: 'Włącz powiadomienia o ponownym połączeniu',
            help: "Wyślij powiadomienie do administratorów po ponownym nawiązaniu połączenia z Gateway'em",
          },
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
          'Na podstawie tego adresu będzie stworzona sieć VPN, np. 10.10.10.1/24 (sieć VPN: 10.10.10.0/24). Opcjonalnie możesz podać wiele adresów, oddzielając je przecinkiem. Pierwszy adres będzie adresem głównym i zostanie użyty do przypisywania adresów IP urządzeniom. Pozostałe adresy są dodatkowe i nie będą zarządzane przez Defguarda.',
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
    header: {
      main: 'Uruchomienie serwera gateway',
      dockerBasedGatewaySetup: `Konfiguracja gateway za pomocą narzędzia docker`,
      fromPackage: `Z pakietu`,
      oneLineInstall: `Instalacja za pomocą jednej linii`,
    },
    card: {
      title: 'Komenda Dockera uruchamiająca serwer gateway',
      authToken: 'Token Autoryzacyjny',
    },
    button: {
      availablePackages: `Dostępne pakiety`,
    },
    controls: {
      status: 'Sprawdź status połączenia',
    },
    messages: {
      runCommand: `Defguard wymaga uruchomienia serwera gateway w celu kontrolowania VPN.
            Szczegóły znajdziesz w [dokumentacji]({setupGatewayDocs}).
            Istnieje wiele sposobów na uruchomienie serwera gateway, poniższy przykład używa technologii Docker,
            więcej przykładów znajdziesz w [dokumentacji]({setupGatewayDocs}).`,
      createNetwork: `Utwórz sieć przed uruchomieniem procesu gateway.`,
      noConnection: `Brak połączenia proszę uruchom poniższą komendę.`,
      connected: `Gateway połączony.`,
      statusError: 'Nie udało się uzyskać statusu',
      oneLineInstall: `Jeśli wykonujesz instalację w jednej linii: https://docs.defguard.net/admin-and-features/setting-up-your-instance/one-line-install
        nie ma potrzeby wykonywania dalszych kroków.`,
      fromPackage: `Zainstaluj pakiet dostępny na https://github.com/DefGuard/gateway/releases/latest i skonfiguruj \`/etc/defguard/gateway.toml\`
        na podstawie [dokumentacji]({setupGatewayDocs}).`,
      authToken: `Poniższy token jest wymagany do autoryzacji i konfiguracji węzła gateway. Upewnij się, że zachowasz ten token w bezpiecznym miejscu,
        a następnie podążaj za instrukcją wdrażania usługi znajdującej się w [dokumentacji]({setupGatewayDocs}), aby pomyślnie skonfigurować serwer gateway.
        Po więcej szczegółów i dokładnych kroków, proszę zapoznaj się z [dokumentacją](setupGatewayDocs).`,
      dockerBasedGatewaySetup: `Poniżej znajduje się przykład oparty na Dockerze.
        Więcej szczegółów i dokładnych kroków można znaleźć w [dokumentacji]({setupGatewayDocs}).`,
    },
  },
  loginPage: {
    pageTitle: 'Wprowadź swoje dane logowania',
    callback: {
      return: 'Powrót do logowania',
      error: 'Wystąpił błąd podczas logowania przez zewnętrznego dostawcę OpenID',
    },
    oidcLogin: 'Zaloguj się przez',
    mfa: {
      title: 'Autoryzacja dwuetapowa.',
      controls: {
        useAuthenticator: 'Zamiast tego użyj aplikacji Authenticator',
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
    completed: 'Sieć skonfigurowana',
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
    subtitle: 'Wkrótce nastąpi przekierowanie...',
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
Przed zgłoszeniem problemów na GitHub należy zapoznać z dokumentacją dostępną na [docs.defguard.net](https://docs.defguard.net/)

Aby zgłosić:
* Problem - przejdź do [GitHub](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=bug&template=bug_report.md&title=)
* Prośbę o nową funkcjonalność - przejdź do [GitHub](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=feature&template=feature_request.md&title=)

W przypadku innych zgłoszeń skontaktuj się z nami: support@defguard.net
`,
    },
  },
  devicesPage: {
    title: 'Urządzenia sieciowe',
    search: {
      placeholder: 'Znajdź',
    },
    bar: {
      itemsCount: 'Wszystkie urządzenia',
      filters: {},
      actions: {
        addNewDevice: 'Dodaj nowe',
      },
    },
    list: {
      columns: {
        labels: {
          name: 'Nazwa',
          location: 'Położenie',
          assignedIp: 'Adres IP',
          description: 'Opis',
          addedBy: 'Dodane przez',
          addedAt: 'Data dodania',
          edit: 'Zmień',
        },
      },
      edit: {
        actionLabels: {
          config: 'Zobacz konfigurację',
          generateToken: 'Utwórz kupon autoryzacyjny',
        },
      },
    },
  },
};

const pl = deepmerge(en, translation);

export default pl;

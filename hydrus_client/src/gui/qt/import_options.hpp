#pragma once

// WS-15.1.3b: import options dialog moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QCheckBox>
#include <QComboBox>
#include <QDialog>
#include <QHash>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QSpinBox>
#include <QString>
#include <QTabWidget>
#include <QTableWidget>
#include <QWidget>

class HydrusMainWindow;


// Editor for the import options containers: per-caller defaults and named
// favourites. One tab per option type, with a "use default" switch so a
// container only overrides what it sets.
class HydrusImportOptionsDialog : public QDialog {
public:
    explicit HydrusImportOptionsDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

private:
    struct Editor {
        QTabWidget *tabs = nullptr;
        QHash<QString, QCheckBox *> use_default;
        QHash<QString, QWidget *> content;

        QComboBox *prefetch_hash = nullptr;
        QComboBox *prefetch_url = nullptr;
        QCheckBox *prefetch_spam = nullptr;
        QCheckBox *prefetch_meta_hash = nullptr;
        QCheckBox *prefetch_meta_url = nullptr;

        QSpinBox *min_size = nullptr;
        QSpinBox *max_size = nullptr;
        QSpinBox *min_width = nullptr;
        QSpinBox *min_height = nullptr;
        QSpinBox *max_width = nullptr;
        QSpinBox *max_height = nullptr;
        QSpinBox *max_gif_size = nullptr;
        QCheckBox *exclude_deleted = nullptr;
        QCheckBox *allow_bombs = nullptr;
        QListWidget *mimes = nullptr;

        QTableWidget *tag_rules = nullptr;
        QPlainTextEdit *tag_whitelist = nullptr;

        QListWidget *destinations = nullptr;
        QCheckBox *auto_archive = nullptr;
        QCheckBox *assoc_primary = nullptr;
        QCheckBox *assoc_source = nullptr;
        QCheckBox *archive_existing = nullptr;
        QCheckBox *destinations_existing = nullptr;

        QTableWidget *tag_services = nullptr;

        QCheckBox *get_notes = nullptr;
        QCheckBox *extend_notes = nullptr;
        QComboBox *note_conflict = nullptr;
        QPlainTextEdit *note_whitelist = nullptr;
        QLineEdit *note_all_override = nullptr;
        QPlainTextEdit *note_overrides = nullptr;

        QComboBox *presentation_status = nullptr;
        QComboBox *presentation_inbox = nullptr;

        QTableWidget *external_programs = nullptr;
    };

    void build_editor();
    void rebuild_list(const QString &select_key = QString());
    void select_key(const QString &key);
    QString current_key() const;
    void store_current();
    void fill_editor(const QJsonObject &container, const QJsonObject &fallback);
    QJsonObject read_editor() const;
    QJsonObject *container_for(const QString &key);
    void add_favourite();
    void rename_favourite();
    void delete_favourite();
    void save();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QWidget *left_panel = nullptr;
    QListWidget *list = nullptr;
    QLabel *description = nullptr;
    QPushButton *rename_button = nullptr;
    QPushButton *delete_button = nullptr;
    Editor editor;
    bool rebuilding = false;
    bool specific_mode = false;
    QString selected_key;
};

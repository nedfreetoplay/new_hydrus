#pragma once

// WS-15.1.2a: HydrusMetadataPanel moved verbatim from bridge.cpp (move-only split).

#include <QJsonObject>
#include <QString>
#include <QStringList>
#include <QWidget>

class QCheckBox;
class QComboBox;
class QCompleter;
class QLineEdit;
class QListWidget;
class QVBoxLayout;
class HydrusMainWindow;

// Full Hydrus-style metadata sidebar: tag services, ratings and notes for the
// focused file. Shared by query pages and the media viewer.
class HydrusMetadataPanel : public QWidget {
public:
    explicit HydrusMetadataPanel(HydrusMainWindow *main, QWidget *parent = nullptr);

    void set_state(const QJsonObject &metadata);
    void set_suggestions(const QStringList &suggestions);

private:
    void rebuild_tags();
    void rebuild_ratings(const QJsonArray &ratings);
    void add_tag();
    void remove_selected_tags();
    void edit_notes();
    void send_tag(const QString &tag, bool add);
    void send_rating(const QString &service, const QString &action, double value = 0.0);

    HydrusMainWindow *main = nullptr;
    QString hash;
    QJsonObject data;
    QComboBox *services = nullptr;
    QLineEdit *tag_filter = nullptr;
    QListWidget *tags = nullptr;
    QCheckBox *display_tags = nullptr;
    QLineEdit *tag_input = nullptr;
    QCompleter *completer = nullptr;
    QWidget *ratings = nullptr;
    QVBoxLayout *ratings_layout = nullptr;
    QListWidget *notes = nullptr;
};

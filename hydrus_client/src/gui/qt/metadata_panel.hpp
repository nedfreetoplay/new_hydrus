#pragma once

// WS-15.1.2a: HydrusMetadataPanel moved verbatim from bridge.cpp (move-only split).
// WS-15.2.2b: contents arrive as a typed MetadataState (see typed_media.cpp);
// the panel keeps plain Qt copies of what it re-renders on toggle.

#include <QList>
#include <QMap>
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

// WS-15.2.2b: MetadataState is owned by Rust (api.rs); only a forward
// declaration is needed here.
struct MetadataState;

// Plain Qt copy of one tag service row: what rebuild_tags re-renders when
// the filter or the display-tags toggle changes.
struct HydrusTagServiceData {
    QString key;
    QString name;
    QStringList tags;
    QStringList display_tags;
    QMap<QString, QStringList> sources;
};

// Plain Qt copy of one rating service row: what rebuild_ratings renders.
// kind follows the cxx RatingKind order (0 numerical, 1 like, 2 incdec).
struct HydrusRatingData {
    QString key;
    QString name;
    int kind = 0;
    double value_num = 0.0;
    QString value_text;
    long long value_int = 0;
};

// Full Hydrus-style metadata sidebar: tag services, ratings and notes for the
// focused file. Shared by query pages and the media viewer.
class HydrusMetadataPanel : public QWidget {
public:
    explicit HydrusMetadataPanel(HydrusMainWindow *main, QWidget *parent = nullptr);

    void set_state(const MetadataState &metadata);
    void set_suggestions(const QStringList &suggestions);

private:
    void rebuild_tags();
    void rebuild_ratings();
    void add_tag();
    void remove_selected_tags();
    void edit_notes();
    void send_tag(const QString &tag, bool add);
    void send_rating(const QString &service, const QString &action, double value = 0.0);

    HydrusMainWindow *main = nullptr;
    QString hash;
    QList<HydrusTagServiceData> tag_services_data;
    QList<HydrusRatingData> ratings_data;
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

#pragma once

// WS-15.1.3c1: file seed cache dialog moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QString>
#include <QStringList>
#include <QVector>

class QComboBox;
class QLabel;
class QLineEdit;
class QPushButton;
class QTableWidget;
class HydrusMainWindow;
struct SeedCacheState;

// WS-15.2.3c: plain Qt copy of one typed seed row for table rebuilding.
struct SeedRowData {
    QString status;
    QString url;
    QString note;
    qint64 created = 0;
    qint64 modified = 0;
};

// File seed cache (urls/gallery/simple pages) and gallery seed log. Both share
// the same table; the gallery log reads the page's gallery seeds instead of the
// file queue.
class HydrusSeedCacheDialog : public QDialog {
public:
    HydrusSeedCacheDialog(HydrusMainWindow *main, const SeedCacheState &state, bool gallery, QWidget *parent = nullptr);

    void set_state(const SeedCacheState &state);

private:
    void rebuild();
    void send_action(const QString &action);
    QStringList selected_urls() const;

    HydrusMainWindow *main = nullptr;
    bool gallery = false;
    QString page_id;
    QString subtitle_text;
    int failed_count = 0;
    QVector<QPair<QString, QString>> statuses;
    QVector<SeedRowData> rows;
    QTableWidget *table = nullptr;
    QComboBox *filter = nullptr;
    QLineEdit *search = nullptr;
    QLabel *subtitle = nullptr;
    QPushButton *retry_button = nullptr;
};

#pragma once

// WS-15.1.3c1: file seed cache dialog moved verbatim from bridge.hpp/bridge.cpp (move-only split).

#include <QDialog>
#include <QJsonObject>
#include <QString>
#include <QStringList>

class QComboBox;
class QLabel;
class QLineEdit;
class QPushButton;
class QTableWidget;
class HydrusMainWindow;

// File seed cache (urls/gallery/simple pages) and gallery seed log. Both share
// the same table; the gallery log reads the page's gallery seeds instead of the
// file queue.
class HydrusSeedCacheDialog : public QDialog {
public:
    HydrusSeedCacheDialog(HydrusMainWindow *main, const QString &json, bool gallery, QWidget *parent = nullptr);

    void set_state(const QString &json);

private:
    void rebuild();
    void send_action(const QString &action);
    QStringList selected_urls() const;

    HydrusMainWindow *main = nullptr;
    bool gallery = false;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QComboBox *filter = nullptr;
    QLineEdit *search = nullptr;
    QLabel *subtitle = nullptr;
    QPushButton *retry_button = nullptr;
};
